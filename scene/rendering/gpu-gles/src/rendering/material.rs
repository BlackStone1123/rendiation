use crate::*;

pub trait GLESModelMaterialRenderImpl {
  fn make_component<'a>(
    &'a self,
    idx: AllocIdx<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>>;
}

impl GLESModelMaterialRenderImpl for Vec<Box<dyn GLESModelMaterialRenderImpl>> {
  fn make_component<'a>(
    &'a self,
    idx: AllocIdx<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    for provider in self {
      if let Some(com) = provider.make_component(idx, cx) {
        return Some(com);
      }
    }
    None
  }
}

#[derive(Default)]
pub struct FlatMaterialDefaultRenderImplProvider {
  uniforms: UpdateResultToken,
}
impl RenderImplProvider<Box<dyn GLESModelMaterialRenderImpl>>
  for FlatMaterialDefaultRenderImplProvider
{
  fn register_resource(&mut self, source: &mut ReactiveStateJoinUpdater, cx: &GPUResourceCtx) {
    let updater = flat_material_uniforms(cx);
    self.uniforms = source.register_multi_updater(updater);
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn GLESModelMaterialRenderImpl> {
    Box::new(FlatMaterialDefaultRenderImpl {
      material_access: global_entity_component_of::<StandardModelRefFlatMaterial>().read(),
      uniforms: res.take_multi_updater_updated(self.uniforms).unwrap(),
    })
  }
}

struct FlatMaterialDefaultRenderImpl {
  material_access: ComponentReadView<StandardModelRefFlatMaterial>,
  uniforms: LockReadGuardHolder<FlatMaterialUniforms>,
}

impl GLESModelMaterialRenderImpl for FlatMaterialDefaultRenderImpl {
  fn make_component<'a>(
    &'a self,
    idx: AllocIdx<StandardModelEntity>,
    _cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let idx = self.material_access.get(idx)?;
    let idx = (*idx)?.index();
    Some(Box::new(FlatMaterialGPU {
      uniform: self.uniforms.get(&idx.into())?,
    }))
  }
}

#[derive(Default)]
pub struct PbrMRMaterialDefaultRenderImplProvider {
  uniforms: UpdateResultToken,
  tex_uniforms: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn GLESModelMaterialRenderImpl>>
  for PbrMRMaterialDefaultRenderImplProvider
{
  fn register_resource(&mut self, source: &mut ReactiveStateJoinUpdater, cx: &GPUResourceCtx) {
    self.uniforms = source.register_multi_updater(pbr_mr_material_uniforms(cx));
    self.tex_uniforms = source.register_multi_updater(pbr_mr_material_tex_uniforms(cx));
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn GLESModelMaterialRenderImpl> {
    Box::new(PbrMRMaterialDefaultRenderImpl {
      material_access: global_entity_component_of::<StandardModelRefPbrMRMaterial>().read(),
      uniforms: res.take_multi_updater_updated(self.uniforms).unwrap(),
      tex_uniforms: res.take_multi_updater_updated(self.tex_uniforms).unwrap(),
      alpha_mode: global_entity_component_of().read(),
      base_color_tex_sampler: TextureSamplerIdView::read_from_global(),
      mr_tex_sampler: TextureSamplerIdView::read_from_global(),
      emissive_tex_sampler: TextureSamplerIdView::read_from_global(),
      normal_tex_sampler: TextureSamplerIdView::read_from_global(),
    })
  }
}

struct PbrMRMaterialDefaultRenderImpl {
  material_access: ComponentReadView<StandardModelRefPbrMRMaterial>,
  uniforms: LockReadGuardHolder<PbrMRMaterialUniforms>,
  tex_uniforms: LockReadGuardHolder<PbrMRMaterialTexUniforms>,
  alpha_mode: ComponentReadView<PbrMRMaterialAlphaModeComponent>,
  base_color_tex_sampler: TextureSamplerIdView<PbrMRMaterialBaseColorTex>,
  mr_tex_sampler: TextureSamplerIdView<PbrMRMaterialMetallicRoughnessTex>,
  emissive_tex_sampler: TextureSamplerIdView<PbrMRMaterialEmissiveTex>,
  normal_tex_sampler: TextureSamplerIdView<NormalTexSamplerOf<PbrMRMaterialNormalInfo>>,
}

pub struct TextureSamplerIdView<T: TextureWithSamplingForeignKeys> {
  pub texture: ComponentReadView<SceneTexture2dRefOf<T>>,
  pub sampler: ComponentReadView<SceneSamplerRefOf<T>>,
}

impl<T: TextureWithSamplingForeignKeys> TextureSamplerIdView<T> {
  pub fn read_from_global() -> Self {
    Self {
      texture: global_entity_component_of().read(),
      sampler: global_entity_component_of().read(),
    }
  }

  pub fn get_pair(&self, id: u32) -> Option<(u32, u32)> {
    let tex = self.texture.get(id.into())?;
    let tex = (*tex)?.index();
    let sampler = self.sampler.get(id.into())?;
    let sampler = (*sampler)?.index();
    Some((tex, sampler))
  }
}

impl GLESModelMaterialRenderImpl for PbrMRMaterialDefaultRenderImpl {
  fn make_component<'a>(
    &'a self,
    idx: AllocIdx<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let idx = self.material_access.get(idx)?;
    let idx = (*idx)?.index();
    let r = PhysicalMetallicRoughnessMaterialGPU {
      uniform: self.uniforms.get(&idx.into())?,
      alpha_mode: self.alpha_mode.get_value(idx.into())?,
      base_color_tex_sampler: self.base_color_tex_sampler.get_pair(idx)?,
      mr_tex_sampler: self.mr_tex_sampler.get_pair(idx)?,
      emissive_tex_sampler: self.emissive_tex_sampler.get_pair(idx)?,
      normal_tex_sampler: self.normal_tex_sampler.get_pair(idx)?,
      texture_uniforms: self.tex_uniforms.get(&idx.into())?,
      binding_sys: cx,
    };
    let r = Box::new(r) as Box<dyn RenderComponent + '_>;
    Some(r)
  }
}
