use crate::*;

pub mod allocator;
pub use allocator::*;

pub mod basic;
pub use basic::*;

pub mod sampling;
pub use sampling::*;

pub struct ShadowMapSystem {
  pub shadow_collections: HashMap<TypeId, Box<dyn ShadowCollection>>,
  pub maps: ShadowMapAllocator,
  pub sampler: RawComparisonSampler,
}

pub trait ShadowCollection: RenderComponentAny + RebuildAbleGPUCollectionBase {
  fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: RenderComponentAny + RebuildAbleGPUCollectionBase + Any> ShadowCollection for T {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
}

impl ShadowMapSystem {
  pub fn new(gpu: &GPU) -> Self {
    let mut sampler = SamplerDescriptor::default();
    sampler.compare = CompareFunction::Less.into();
    Self {
      shadow_collections: Default::default(),
      maps: Default::default(),
      sampler: gpu.device.create_and_cache_com_sampler(sampler),
    }
  }

  pub fn before_update_scene(&mut self, _gpu: &GPU) {
    self
      .shadow_collections
      .iter_mut()
      .for_each(|(_, c)| c.reset());
  }

  pub fn after_update_scene(&mut self, gpu: &GPU) {
    self.shadow_collections.iter_mut().for_each(|(_, c)| {
      c.update_gpu(gpu);
    });
  }
}

impl ShaderPassBuilder for ShadowMapSystem {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    for impls in self.shadow_collections.values() {
      impls.setup_pass(ctx)
    }
    self.maps.setup_pass(ctx)
  }
}

impl ShaderHashProvider for ShadowMapSystem {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    for impls in self.shadow_collections.values() {
      impls.hash_pipeline(hasher)
    }
    // self.maps.hash_pipeline(ctx) // we don't need this now
  }
}

impl ShaderGraphProvider for ShadowMapSystem {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    for impls in self.shadow_collections.values() {
      impls.build(builder)?;
    }
    self.maps.build(builder)
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct)]
pub struct BasicShadowMapInfo {
  pub shadow_camera: CameraGPUTransform,
  pub bias: ShadowBias,
  pub map_info: ShadowMapAddressInfo,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct)]
pub struct ShadowBias {
  pub bias: f32,
  pub normal_bias: f32,
}

impl ShadowBias {
  pub fn new(bias: f32, normal_bias: f32) -> Self {
    Self {
      bias,
      normal_bias,
      ..Zeroable::zeroed()
    }
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct ShadowMapAddressInfo {
  pub layer_index: i32,
  pub size: Vec2<f32>,
  pub offset: Vec2<f32>,
}

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct LightShadowAddressInfo {
  pub index: u32,
  pub enabled: u32,
}

impl LightShadowAddressInfo {
  pub fn new(enabled: bool, index: u32) -> Self {
    Self {
      enabled: enabled.into(),
      index,
      ..Zeroable::zeroed()
    }
  }
}

pub fn compute_shadow_position(
  builder: &ShaderGraphFragmentBuilderView,
  shadow_info: ENode<BasicShadowMapInfo>,
) -> Result<Node<Vec3<f32>>, ShaderGraphBuildError> {
  // another way to compute this is in vertex shader, maybe we will try it later.
  let bias = shadow_info.bias.expand();
  let world_position = builder.query::<FragmentWorldPosition>()?;
  let world_normal = builder.query::<FragmentWorldNormal>()?;

  // apply normal bias
  let world_position = world_position + bias.normal_bias * world_normal;

  let shadow_position =
    shadow_info.shadow_camera.expand().view_projection * (world_position, 1.).into();

  let shadow_position = shadow_position.xyz() / shadow_position.w();

  // convert to uv space and apply offset bias
  Ok(
    shadow_position * consts(Vec3::new(0.5, -0.5, 1.))
      + consts(Vec3::new(0.5, 0.5, 0.))
      + (0., 0., bias.bias).into(),
  )
}
