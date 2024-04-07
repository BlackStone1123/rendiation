use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct SpotLightShaderInfo {
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_cos: f32,
  pub half_penumbra_cos: f32,
}

pub fn spot_uniform_array(
  gpu: &GPUResourceCtx,
) -> UniformArrayUpdateContainer<SpotLightShaderInfo> {
  let buffer = UniformBufferDataView::create_default(&gpu.device);

  let luminance_intensity = global_watch()
    .watch_typed_key::<SplitLightIntensity>()
    .into_uniform_array_collection_update(
      offset_of!(SpotLightShaderInfo, luminance_intensity),
      gpu,
    );

  let cutoff_distance = global_watch()
    .watch_typed_key::<SpotLightCutOffDistance>()
    .into_uniform_array_collection_update(offset_of!(SpotLightShaderInfo, cutoff_distance), gpu);

  let half_cone_cos = global_watch()
    .watch_typed_key::<SpotLightHalfConeAngle>()
    .collective_map(|rad| rad.cos())
    .into_uniform_array_collection_update(offset_of!(SpotLightShaderInfo, half_cone_cos), gpu);

  let half_penumbra_cos = global_watch()
    .watch_typed_key::<SpotLightHalfPenumbraAngle>()
    .collective_map(|rad| rad.cos())
    .into_uniform_array_collection_update(offset_of!(SpotLightShaderInfo, half_penumbra_cos), gpu);

  // todo

  UniformArrayUpdateContainer::new(buffer)
    .with_source(luminance_intensity)
    .with_source(cutoff_distance)
    .with_source(half_cone_cos)
    .with_source(half_penumbra_cos)
}
