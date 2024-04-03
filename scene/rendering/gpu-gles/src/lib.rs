use database::*;
use reactive::*;
use rendiation_scene_core_next::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_reactive_utils::*;

mod material;
pub use material::*;
mod camera;
pub use camera::*;
mod node;
pub use node::*;

pub fn global_watch() -> DatabaseMutationWatch {
  todo!()
}

pub fn global_rev_ref() -> DatabaseEntityReverseReference {
  todo!()
}
