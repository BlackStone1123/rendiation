pub mod forward;
pub use forward::*;

pub mod list;
pub use list::*;

pub mod copy_frame;
pub use copy_frame::*;
pub mod highlight;
pub use highlight::*;
pub mod background;
pub use background::*;
pub mod utils;
use rendiation_webgpu::{BindingBuilder, GPURenderPass};
pub use utils::*;

pub mod framework;
pub use framework::*;

use crate::{GPUResourceCache, SourceOfRendering};

pub struct SceneRenderPass<'a, 'b> {
  pub pass: &'b mut GPURenderPass<'a>,
  pub binding: BindingBuilder,
  pub resources: &'b GPUResourceCache,
}

impl<'a, 'b> std::ops::Deref for SceneRenderPass<'a, 'b> {
  type Target = GPURenderPass<'a>;

  fn deref(&self) -> &Self::Target {
    self.pass
  }
}

impl<'a, 'b> std::ops::DerefMut for SceneRenderPass<'a, 'b> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.pass
  }
}
