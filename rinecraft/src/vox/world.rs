use crate::vox::block::Block;
use crate::vox::block::BlockFace;
use crate::vox::chunk::*;
use crate::vox::util::*;
use crate::{
  shading::{create_block_shading, BlockShadingParamGroup},
  util::CameraGPU,
  vox::world_machine::*,
};
use render_target::TargetStates;
use rendiation_math::*;
use rendiation_render_entity::{PerspectiveCamera, TransformedObject};
use rendiation_scenegraph::*;
use rendiation_webgpu::*;
use std::collections::HashMap;
use std::collections::{BTreeMap, HashSet};

pub struct World {
  pub world_machine: WorldMachineImpl,
  pub chunk_visible_distance: usize,
  pub chunks: HashMap<(i32, i32), Chunk>,
  pub chunk_geometry_update_set: HashSet<(i32, i32)>,
  scene_data: Option<WorldSceneAttachment>,
}

struct WorldSceneAttachment {
  root_node_index: SceneNodeHandle<WebGPUBackend>,
  block_shading: ShadingHandle<WebGPUBackend>,
  blocks: BTreeMap<
    (i32, i32),
    (
      SceneNodeHandle<WebGPUBackend>,
      RenderObjectHandle<WebGPUBackend>,
      GeometryHandle<WebGPUBackend>,
    ),
  >,
}

impl World {
  pub fn new() -> Self {
    let chunks = HashMap::new();
    World {
      chunk_visible_distance: 4,
      chunks,
      chunk_geometry_update_set: HashSet::new(),
      world_machine: WorldMachineImpl::new(),
      scene_data: None,
    }
  }

  pub fn attach_scene(
    &mut self,
    scene: &mut Scene<WebGPUBackend>,
    renderer: &mut WGPURenderer,
    camera_gpu: &CameraGPU,
    target: &TargetStates,
  ) {
    if self.scene_data.is_some() {
      return;
    }

    let block_atlas = self.world_machine.get_block_atlas(renderer);
    let sampler = WGPUSampler::default(renderer);

    let shading_params = BlockShadingParamGroup {
      texture_view: &block_atlas.view(),
      sampler: &sampler,
      u_mvp_matrix: scene
        .resources
        .get_uniform(camera_gpu.gpu_mvp_matrix)
        .resource(),
      u_camera_world_position: scene
        .resources
        .get_uniform(camera_gpu.gpu_camera_position)
        .resource(),
    }
    .create_bindgroup(renderer);

    let block_shading = create_block_shading(renderer, target);
    let bindgroup_index = scene
      .resources
      .add_shading_param_group(SceneShadingParameterGroupData::new(ParameterGroupTypeId(0), shading_params))
      .index();
    let block_shading = scene
      .resources
      .add_shading(SceneShadingData::new(block_shading).push_parameter(bindgroup_index));
    let block_shading = block_shading.index();

    let root_node_index = scene.create_new_node().handle();
    scene.add_to_scene_root(root_node_index);

    self.scene_data = Some(WorldSceneAttachment {
      root_node_index,
      block_shading,
      blocks: BTreeMap::new(),
    })
  }

  pub fn detach_scene(&mut self) {
    // free the resource in scene
    todo!()
  }

  pub fn assure_chunk(
    world_machine: &mut impl WorldMachine,
    chunks: &mut HashMap<(i32, i32), Chunk>,
    chunk_key: (i32, i32),
  ) -> bool {
    let mut exist = true;
    chunks.entry(chunk_key).or_insert_with(|| {
      println!("chunk generate {:?}", chunk_key);
      exist = false;
      Chunk::new(chunk_key, world_machine)
    });
    exist
  }

  pub fn update(&mut self, renderer: &mut WGPURenderer, scene: &mut Scene<WebGPUBackend>) {
    let camera = scene.cameras.get_active_camera_mut::<PerspectiveCamera>();
    let camera_position = camera.world_matrix.position();

    let stand_point_chunk = query_point_in_chunk(camera_position);
    let x_low = stand_point_chunk.0 - self.chunk_visible_distance as i32;
    let x_high = stand_point_chunk.0 + self.chunk_visible_distance as i32;
    let z_low = stand_point_chunk.1 - self.chunk_visible_distance as i32;
    let z_high = stand_point_chunk.1 + self.chunk_visible_distance as i32;
    let mut create_list = Vec::new();
    for x in x_low..x_high {
      for z in z_low..z_high {
        if !World::assure_chunk(&mut self.world_machine, &mut self.chunks, (x, z)) {
          create_list.push((x, z));
        }
        if let Some(scene_data) = &mut self.scene_data {
          if !scene_data.blocks.contains_key(&(x, z)) {
            create_list.push((x, z));
          }
        }
      }
    }

    // dispatch change to adjacent chunk
    for chunk_key in create_list {
      self.chunk_geometry_update_set.insert(chunk_key);
      World::assure_chunk(
        &mut self.world_machine,
        &mut self.chunks,
        (chunk_key.0 + 1, chunk_key.1),
      );
      World::assure_chunk(
        &mut self.world_machine,
        &mut self.chunks,
        (chunk_key.0 - 1, chunk_key.1),
      );
      World::assure_chunk(
        &mut self.world_machine,
        &mut self.chunks,
        (chunk_key.0, chunk_key.1 + 1),
      );
      World::assure_chunk(
        &mut self.world_machine,
        &mut self.chunks,
        (chunk_key.0, chunk_key.1 - 1),
      );
    }

    // sync change to scene
    if let Some(scene_data) = &mut self.scene_data {
      for chunk_to_update_key in &self.chunk_geometry_update_set {
        // remove node in scene;
        if let Some((node_index, render_object_index, geometry_index)) =
          scene_data.blocks.get(chunk_to_update_key)
        {
          scene.node_remove_child_by_handle(scene_data.root_node_index, *node_index);
          scene.free_node(*node_index);
          scene.delete_render_object(*render_object_index);
          scene
            .resources
            .delete_geometry_with_buffers(*geometry_index);
          scene_data.blocks.remove(chunk_to_update_key);
        }

        // add new node in scene;
        let mesh_buffer =
          Chunk::create_mesh_buffer(&self.world_machine, &self.chunks, *chunk_to_update_key);
        let scene_geometry = Chunk::create_add_geometry(&mesh_buffer, renderer, scene);

        let render_object_index =
          scene.create_render_object(scene_geometry, scene_data.block_shading);
        let new_node = scene.create_new_node();
        new_node.data_mut().add_render_object(render_object_index);
        let node_index = new_node.handle();

        scene.node_add_child_by_handle(scene_data.root_node_index, node_index);

        scene_data.blocks.insert(
          *chunk_to_update_key,
          (node_index, render_object_index, scene_geometry),
        );
      }
    }
    self.chunk_geometry_update_set.clear();
  }

  pub fn try_get_block(
    chunks: &HashMap<(i32, i32), Chunk>,
    block_position: &Vec3<i32>,
  ) -> Option<Block> {
    let chunk_position = query_block_in_chunk(block_position);
    let chunk_op = chunks.get(&chunk_position);
    if let Some(chunk) = chunk_op {
      let chunk_local_position = get_local_block_position(block_position);
      Some(chunk.get_block(chunk_local_position))
    } else {
      None
    }
  }

  pub fn check_block_face_visibility(
    chunks: &HashMap<(i32, i32), Chunk>,
    block_position: &Vec3<i32>,
    face: BlockFace,
  ) -> bool {
    if let Some(opposite_position) = World::block_face_opposite_position(*block_position, face) {
      if let Some(block) = World::try_get_block(chunks, &opposite_position) {
        if block.is_void() {
          // this is verbose but clear
          true // surface
        } else {
          false // inner
        }
      } else {
        false // chunk edge
      }
    } else {
      true // top bottom world of world
    }
  }

  pub fn block_face_opposite_position(
    block_position: Vec3<i32>,
    face: BlockFace,
  ) -> Option<Vec3<i32>> {
    let mut result = block_position;
    match face {
      BlockFace::XZMin => result.y -= 1,
      BlockFace::XZMax => result.y += 1,
      BlockFace::XYMin => result.z -= 1,
      BlockFace::XYMax => result.z += 1,
      BlockFace::YZMin => result.x -= 1,
      BlockFace::YZMax => result.x += 1,
    };

    if result.y < 0 {
      return None;
    }

    if result.y >= CHUNK_HEIGHT as i32 {
      return None;
    }
    Some(result)
  }
}
