use std::ops::Deref;

use futures::StreamExt;
use reactive::{RemoveToken, SignalStreamExt};
use tree::{AbstractParentAddressableTreeNode, CoreTree, TreeMutation};

use crate::*;

/// compare to scene inner delta, this mixed delta support multi scene content mixing
#[derive(Clone)]
#[allow(non_camel_case_types)]
pub enum MixSceneDelta {
  background(DeltaOf<Option<SceneBackGround>>),
  default_camera(DeltaOf<SceneCamera>),
  active_camera(DeltaOf<Option<SceneCamera>>),
  cameras(ContainerRefRetainContentDelta<SceneCamera>),
  lights(ContainerRefRetainContentDelta<SceneLight>),
  models(ContainerRefRetainContentDelta<SceneModel>),
  ext(DeltaOf<DynamicExtension>),
}

pub fn map_scene_delta_to_mixed(
  input: impl Stream<Item = SceneInnerDelta> + Unpin,
) -> impl Stream<Item = MixSceneDelta> {
  let input = input.create_broad_caster();

  let cameras = input
    .fork_stream()
    .filter_map_sync(|delta| match delta {
      SceneInnerDelta::cameras(c) => Some(c),
      _ => None,
    })
    .map(IndependentItemContainerDelta::from)
    .transform_delta_to_ref_retained_by_hashing()
    .transform_ref_retained_to_ref_retained_content_by_hashing()
    .map(MixSceneDelta::cameras);

  let lights = input
    .fork_stream()
    .filter_map_sync(|delta| match delta {
      SceneInnerDelta::lights(c) => Some(c),
      _ => None,
    })
    .map(IndependentItemContainerDelta::from)
    .transform_delta_to_ref_retained_by_hashing()
    .transform_ref_retained_to_ref_retained_content_by_hashing()
    .map(MixSceneDelta::lights);

  let models = input
    .fork_stream()
    .filter_map_sync(|delta| match delta {
      SceneInnerDelta::models(c) => Some(c),
      _ => None,
    })
    .map(IndependentItemContainerDelta::from)
    .transform_delta_to_ref_retained_by_hashing()
    .transform_ref_retained_to_ref_retained_content_by_hashing()
    .map(MixSceneDelta::models);

  let others = input.fork_stream().filter_map_sync(|delta| match delta {
    SceneInnerDelta::background(b) => MixSceneDelta::background(b).into(),
    SceneInnerDelta::default_camera(c) => MixSceneDelta::default_camera(c).into(),
    SceneInnerDelta::active_camera(c) => MixSceneDelta::active_camera(c).into(),
    SceneInnerDelta::ext(ext) => MixSceneDelta::ext(ext).into(),
    _ => None,
  });

  let output = futures::stream::select(cameras, lights);
  let output = futures::stream::select(output, models);
  futures::stream::select(output, others)
}

pub fn mix_scene_folding(
  input: impl Stream<Item = MixSceneDelta>,
) -> (
  impl Stream<Item = MixSceneDelta>,
  (Scene, SceneNodeDeriveSystem),
) {
  let (scene, derives) = SceneInner::new();

  let s = scene.clone();
  let nodes = scene.read().nodes.clone();
  let rebuilder = SceneRebuilder::new(nodes);
  let rebuilder = Arc::new(RwLock::new(rebuilder));
  let mut model_handle_map: HashMap<usize, SceneModelHandle> = HashMap::new();
  let mut camera_handle_map: HashMap<usize, SceneCameraHandle> = HashMap::new();
  let mut light_handle_map: HashMap<usize, SceneLightHandle> = HashMap::new();

  let output = input.map(move |delta| {
    //
    match &delta {
      MixSceneDelta::background(bg) => {
        SceneInnerDelta::background(bg.clone()).apply_modify(&s);
      }
      MixSceneDelta::default_camera(_) => {}
      MixSceneDelta::active_camera(camera) => {
        let mapped_camera = camera
          .as_ref()
          .map(merge_maybe_ref)
          .map(|camera| {
            let mapped_camera = camera_handle_map.get(&camera.guid()).unwrap();
            s.read().cameras.get(*mapped_camera).unwrap().clone()
          })
          .map(MaybeDelta::All);

        SceneInnerDelta::active_camera(mapped_camera).apply_modify(&s);
      }
      MixSceneDelta::cameras(camera) => match camera {
        ContainerRefRetainContentDelta::Remove(camera) => {
          let (_, remover) = make_add_remover(&rebuilder);
          remover(camera.read().node.clone());
          let handle = camera_handle_map.remove(&camera.guid()).unwrap();
          s.remove_camera(handle);
        }
        ContainerRefRetainContentDelta::Insert(camera) => {
          let new = transform_camera_node(camera, &rebuilder);
          let new_handle = s.insert_camera(new);
          camera_handle_map.insert(camera.guid(), new_handle);
        }
      },
      MixSceneDelta::lights(light) => match light {
        ContainerRefRetainContentDelta::Remove(light) => {
          let (_, remover) = make_add_remover(&rebuilder);
          remover(light.read().node.clone());
          let handle = light_handle_map.remove(&light.guid()).unwrap();
          s.remove_light(handle);
        }
        ContainerRefRetainContentDelta::Insert(light) => {
          let new = transform_light_node(light, &rebuilder);
          let new_handle = s.insert_light(new);
          light_handle_map.insert(light.guid(), new_handle);
        }
      },
      MixSceneDelta::models(model) => match model {
        ContainerRefRetainContentDelta::Remove(model) => {
          let (_, remover) = make_add_remover(&rebuilder);
          remover(model.read().node.clone());
          let handle = model_handle_map.remove(&model.guid()).unwrap();
          s.remove_model(handle)
        }
        ContainerRefRetainContentDelta::Insert(model) => {
          // todo, should we check the inserted model has been mapped?
          let new = transform_model_node(model, &rebuilder);
          let new_handle = s.insert_model(new);
          model_handle_map.insert(model.guid(), new_handle);
        }
      },
      MixSceneDelta::ext(ext) => {
        SceneInnerDelta::ext(ext.clone()).apply_modify(&s);
      }
    }

    delta
  });

  (output, (scene, derives))
}

fn make_add_remover(
  rebuilder: &ShareableRebuilder,
) -> (
  impl Fn(SceneNode) -> SceneNode + Send + Sync + 'static, // todo, make input param pass by ref
  impl Fn(SceneNode) + Send + Sync + 'static,
) {
  let rebuilder = rebuilder.clone();
  let rebuilder2 = rebuilder.clone();
  let adder = move |node| add_entity_used_node(&rebuilder, &node);
  let remover = move |node| remove_entity_used_node(&rebuilder2, &node);
  (adder, remover)
}

fn transform_camera_node(m: &SceneCamera, rebuilder: &ShareableRebuilder) -> SceneCamera {
  let (adder, remover) = make_add_remover(rebuilder);

  let camera = m.read();
  let r = SceneCameraInner {
    bounds: camera.bounds,
    projection: camera.projection.clone(),
    node: adder(camera.node.clone()),
  }
  .into_ref();

  let mut previous_node = camera.node.clone();

  m.pass_changes_to(&r, move |delta| match delta {
    SceneCameraInnerDelta::node(node) => {
      remover(previous_node.clone());
      previous_node = node.clone();
      SceneCameraInnerDelta::node(adder(node))
    }
    _ => delta,
  });
  r
}

fn transform_light_node(m: &SceneLight, rebuilder: &ShareableRebuilder) -> SceneLight {
  let (adder, remover) = make_add_remover(rebuilder);

  let light = m.read();
  let r = SceneLightInner {
    node: adder(light.node.clone()),
    light: light.light.clone(),
  }
  .into_ref();

  let mut previous_node = light.node.clone();

  m.pass_changes_to(&r, move |delta| match delta {
    SceneLightInnerDelta::node(node) => {
      remover(previous_node.clone());
      previous_node = node.clone();
      SceneLightInnerDelta::node(adder(node))
    }
    _ => delta,
  });
  r
}

fn transform_model_node(m: &SceneModel, rebuilder: &ShareableRebuilder) -> SceneModel {
  let (adder, remover) = make_add_remover(rebuilder);

  let model = m.read();
  let r = SceneModelImpl {
    node: adder(model.node.clone()),
    model: model.model.clone(),
  }
  .into_ref();

  let mut previous_node = model.node.clone();

  m.pass_changes_to(&r, move |delta| match delta {
    SceneModelImplDelta::node(node) => {
      remover(previous_node.clone());
      previous_node = node.clone();
      SceneModelImplDelta::node(adder(node))
    }
    _ => delta,
  });
  r
}

type NodeArenaIndex = usize;
type NodeGuid = usize;
type SceneGuid = usize;

struct NodeMapping {
  mapped: SceneNode,
  sub_tree_entity_ref_count: usize,
}

struct SceneWatcher {
  change_remove_token: RemoveToken<TreeMutation<SceneNodeDataImpl>>,
  ref_count: usize,
  nodes: SceneNodeCollection, // todo weak?
}

impl Drop for SceneWatcher {
  fn drop(&mut self) {
    let inner = self.nodes.inner.inner.read().unwrap();
    inner.source.off(self.change_remove_token);
  }
}

type ShareableRebuilder = Arc<RwLock<SceneRebuilder>>;

struct SceneRebuilder {
  nodes: HashMap<NodeGuid, NodeMapping>,
  id_mapping: HashMap<(SceneGuid, NodeArenaIndex), NodeGuid>,
  scenes: HashMap<SceneGuid, SceneWatcher>,
  target_collection: SceneNodeCollection,
}

impl SceneRebuilder {
  pub fn new(target_collection: SceneNodeCollection) -> Self {
    Self {
      nodes: Default::default(),
      scenes: Default::default(),
      id_mapping: Default::default(),
      target_collection,
    }
  }
}

fn add_watch_origin_scene_change(rebuilder: &ShareableRebuilder, source_node: &SceneNode) {
  let mut rebuilder_mut = rebuilder.write().unwrap();
  let scene_guid = source_node.scene_id;

  let scene_watcher = rebuilder_mut.scenes.entry(scene_guid).or_insert_with(|| {
    let source_collection = SceneNodeCollection {
      inner: source_node.inner.inner.read().unwrap().nodes.clone(),
      scene_guid,
    };
    let source_collection_c = source_collection.clone();
    let rebuilder = rebuilder.clone();

    let remove_token = source_node.inner.visit_raw_storage(move |tree| {
      tree.source.on(move |delta| {
        let mut rebuilder = rebuilder.write().unwrap();

        match delta {
          tree::TreeMutation::Attach {
            parent_target,
            node,
          } => {
            let parent_guid = rebuilder.get_mapped_node_guid(scene_guid, *parent_target);
            let node_guid = rebuilder.get_mapped_node_guid(scene_guid, *node);
            rebuilder.handle_attach(node_guid, parent_guid, *parent_target, &source_collection);
          }
          tree::TreeMutation::Detach { node } => {
            let node_guid = rebuilder.get_mapped_node_guid(scene_guid, *node);
            rebuilder.handle_detach(node_guid, *node, &source_collection);
          }
          tree::TreeMutation::Mutate { node, delta } => {
            // get the mapped node
            let node = rebuilder.get_mapped_node_guid(scene_guid, *node);
            let node = &rebuilder.nodes.get(&node).unwrap().mapped;
            // pass the delta
            node.mutate(|mut n| n.modify(delta.clone()))
          }
          _ => {}
        }
        false
      })
    });

    SceneWatcher {
      change_remove_token: remove_token,
      ref_count: 0, // will increase later
      nodes: source_collection_c,
    }
  });

  scene_watcher.ref_count += 1;
}

fn remove_watch_origin_scene_change(rebuilder: &ShareableRebuilder, node: &SceneNode) {
  let mut rebuilder_mut = rebuilder.write().unwrap();
  let scene_watcher = rebuilder_mut.scenes.get_mut(&node.scene_id).unwrap();

  assert!(scene_watcher.ref_count >= 1);
  scene_watcher.ref_count -= 1;

  if scene_watcher.ref_count == 0 {
    rebuilder_mut.scenes.remove(&node.scene_id);
  }
}

fn add_entity_used_node(rebuilder: &ShareableRebuilder, to_add_node: &SceneNode) -> SceneNode {
  let node = {
    rebuilder
      .write()
      .unwrap()
      .add_entity_used_node_impl(to_add_node)
  };
  add_watch_origin_scene_change(rebuilder, to_add_node);
  node
}

fn remove_entity_used_node(rebuilder: &ShareableRebuilder, to_remove_node: &SceneNode) {
  rebuilder
    .write()
    .unwrap()
    .remove_entity_used_node_impl(to_remove_node);
  remove_watch_origin_scene_change(rebuilder, to_remove_node)
}

impl SceneRebuilder {
  fn handle_attach(
    &mut self,
    child_node_guid: NodeGuid,
    parent_guid: NodeGuid,
    parent_id: NodeArenaIndex,
    source_nodes: &SceneNodeCollection,
  ) {
    let child_sub_tree_entity_ref_count = self
      .nodes
      .get(&child_node_guid)
      .unwrap()
      .sub_tree_entity_ref_count;

    self.check_insert_and_update_parents_entity_ref_count(
      source_nodes,
      parent_id,
      child_sub_tree_entity_ref_count,
    );

    let child = self.nodes.get(&child_node_guid).unwrap();

    let parent = self.nodes.get(&parent_guid).unwrap();
    child.mapped.attach_to(&parent.mapped);
  }

  fn handle_detach(
    &mut self,
    node_guid: NodeGuid,
    node_id: NodeArenaIndex,
    source_nodes: &SceneNodeCollection,
  ) {
    let child = self.nodes.get(&node_guid).unwrap();
    child.mapped.detach_from_parent();

    self.decrease_parent_chain_entity_ref_count_and_check_delete(
      source_nodes,
      node_id,
      child.sub_tree_entity_ref_count,
    );
  }

  fn check_insert_and_update_parents_entity_ref_count(
    &mut self,
    source_nodes: &SceneNodeCollection,
    node_handle: NodeArenaIndex,
    ref_add_count: usize,
  ) {
    let mut child_to_attach = None;
    let source_scene_guid = source_nodes.scene_guid;

    visit_self_parent_chain(
      source_nodes,
      node_handle,
      |node_guid, node_id, node_data| {
        let NodeMapping {
          sub_tree_entity_ref_count,
          ..
        } = self.nodes.entry(node_guid).or_insert_with(|| {
          let mapped = self.target_collection.create_node(node_data.clone());

          self
            .id_mapping
            .insert((source_scene_guid, node_id), mapped.guid());
          child_to_attach = Some(node_guid);

          NodeMapping {
            mapped,
            // will be increased later
            sub_tree_entity_ref_count: 0,
          }
        });

        *sub_tree_entity_ref_count += ref_add_count;

        if let Some(child) = child_to_attach.take() {
          let mapping = self.nodes.get(&child).unwrap();
          let child = mapping.mapped.clone();
          let child_ref_count = mapping.sub_tree_entity_ref_count;

          let mapping = self.nodes.get_mut(&node_guid).unwrap();
          child.attach_to(&mapping.mapped);
          mapping.sub_tree_entity_ref_count += child_ref_count;
        }
      },
    )
  }

  fn decrease_parent_chain_entity_ref_count_and_check_delete(
    &mut self,
    nodes: &SceneNodeCollection,
    node_handle: NodeArenaIndex,
    ref_decrease_count: usize,
  ) {
    let source_scene_guid = nodes.scene_guid;

    visit_self_parent_chain(nodes, node_handle, |node_guid, node_id, _node_data| {
      let mapping = self.nodes.get_mut(&node_guid).unwrap();

      assert!(mapping.sub_tree_entity_ref_count >= ref_decrease_count);
      mapping.sub_tree_entity_ref_count -= ref_decrease_count;

      if mapping.sub_tree_entity_ref_count == 0 {
        self.nodes.remove(&node_guid);
        self.id_mapping.remove(&(source_scene_guid, node_id));
      }
    })
  }

  fn get_mapped_node_guid(&self, scene_id: SceneGuid, index: NodeArenaIndex) -> NodeGuid {
    *self.id_mapping.get(&(scene_id, index)).unwrap()
  }

  fn add_entity_used_node_impl(&mut self, to_add_node: &SceneNode) -> SceneNode {
    let source_nodes = to_add_node.get_node_collection();
    self.check_insert_and_update_parents_entity_ref_count(
      &source_nodes,
      to_add_node.raw_handle().index(),
      1,
    );
    self.nodes.get(&to_add_node.guid()).unwrap().mapped.clone()
  }

  fn remove_entity_used_node_impl(&mut self, to_remove_node: &SceneNode) {
    let source_nodes = to_remove_node.get_node_collection();
    self.decrease_parent_chain_entity_ref_count_and_check_delete(
      &source_nodes,
      to_remove_node.raw_handle().index(),
      1,
    )
  }
}

fn visit_self_parent_chain(
  nodes: &SceneNodeCollection,
  node_handle: NodeArenaIndex,
  mut f: impl FnMut(NodeGuid, NodeArenaIndex, &SceneNodeDataImpl),
) {
  let tree = nodes.inner.inner.read().unwrap();
  let node_handle = tree.inner.recreate_handle(node_handle);

  tree
    .inner
    .create_node_ref(node_handle)
    .traverse_parent(|node| {
      let data = node.node.data();
      let index = node.node.handle().index();
      f(data.guid(), index, data.deref());
      true
    })
}
