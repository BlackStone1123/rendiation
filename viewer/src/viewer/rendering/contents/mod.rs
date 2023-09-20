use crate::*;

mod solid_lined_mesh;
pub use solid_lined_mesh::*;
mod widened_line;
pub use widened_line::*;
mod model_overrides;
pub use model_overrides::*;
use rendiation_mesh_core::{
  vertex::Vertex, DynIndexContainer, GroupedMesh, IndexedMesh, IntersectAbleGroupedMesh,
  TriangleList,
};

pub fn register_viewer_extra_scene_features() {
  register_material::<SharedIncrementalSignal<WidenedLineMaterial>>();

  register_mesh::<SharedIncrementalSignal<SolidLinedMesh>>();
  register_mesh::<SharedIncrementalSignal<WidenedLineMesh>>();
  register_mesh::<
    SharedIncrementalSignal<GroupedMesh<IndexedMesh<TriangleList, Vec<Vertex>, DynIndexContainer>>>,
  >();
}

fn register_mesh<T>()
where
  T: AsRef<dyn GlobalIdentified>
    + AsMut<dyn GlobalIdentified>
    + AsRef<dyn WebGPUSceneMesh>
    + AsMut<dyn WebGPUSceneMesh>
    + AsRef<dyn IntersectAbleGroupedMesh>
    + AsMut<dyn IntersectAbleGroupedMesh>
    // + AsRef<dyn WatchableSceneMeshLocalBounding>
    // + AsMut<dyn WatchableSceneMeshLocalBounding>
    + 'static,
{
  register_core_mesh_features::<T>();
  register_webgpu_mesh_features::<T>();
}

fn register_material<T>()
where
  T: AsRef<dyn GlobalIdentified>
    + AsMut<dyn GlobalIdentified>
    + AsRef<dyn WebGPUSceneMaterial>
    + AsMut<dyn WebGPUSceneMaterial>
    + 'static,
{
  register_core_material_features::<T>();
  register_webgpu_material_features::<T>();
}
