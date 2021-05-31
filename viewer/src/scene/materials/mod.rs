pub mod basic;
use arena::Arena;
pub use basic::*;
use rendiation_algebra::Mat4;

use crate::Renderer;

use super::{Camera, CameraBindgroup, RenderStyle, SceneMesh, SceneSampler, SceneTexture2D};

pub trait BindableResource {
  fn as_bindable(&self) -> wgpu::BindingResource;
}

impl BindableResource for wgpu::Buffer {
  fn as_bindable(&self) -> wgpu::BindingResource {
    self.as_entire_binding()
  }
}

pub trait MaterialCPUResource {
  type GPU;
  fn create<S>(
    &mut self,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<S>,
  ) -> Self::GPU;
}

pub trait MaterialGPUResource<S>: Sized {
  type Source: MaterialCPUResource<GPU = Self>;
  fn update(
    &mut self,
    source: &Self::Source,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<S>,
  ) {
    // default do nothing
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a, S>,
  ) {
    // default do nothing
  }
}

pub struct MaterialCell<T>
where
  T: MaterialCPUResource,
{
  material: T,
  gpu: T::GPU,
}

pub struct SceneMaterialRenderPrepareCtx<'a, S> {
  pub active_camera: &'a Camera,
  pub camera_gpu: &'a CameraBindgroup,
  pub model_matrix: &'a Mat4<f32>,
  pub pipelines: &'a mut PipelineResourceManager,
  pub style: &'a S,
  pub active_mesh: &'a SceneMesh,
  pub textures: &'a mut Arena<SceneTexture2D>,
  pub samplers: &'a mut Arena<SceneSampler>,
}

pub struct SceneMaterialPassSetupCtx<'a, S> {
  pub pipelines: &'a PipelineResourceManager,
  pub camera_gpu: &'a CameraBindgroup,
  pub style: &'a S,
}

pub trait MaterialStyleAbility<S: RenderStyle> {
  fn update<'a>(&mut self, renderer: &Renderer, ctx: &mut SceneMaterialRenderPrepareCtx<'a, S>);
  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a, S>,
  );
}

impl<T, S> MaterialStyleAbility<S> for MaterialCell<T>
where
  T: MaterialCPUResource,
  T::GPU: MaterialGPUResource<S, Source = T>,
  S: RenderStyle,
{
  fn update<'a>(&mut self, renderer: &Renderer, ctx: &mut SceneMaterialRenderPrepareCtx<'a, S>) {
    self.gpu.update(&self.material, renderer, ctx);
  }
  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a, S>,
  ) {
    self.gpu.setup_pass(pass, ctx)
  }
}

pub struct PipelineResourceManager {
  pub basic: Option<wgpu::RenderPipeline>,
}

impl PipelineResourceManager {
  pub fn new() -> Self {
    Self { basic: None }
  }
}
