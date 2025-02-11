use std::any::TypeId;

use crate::*;

pub struct FrameGeneralMaterialBuffer {
  /// the following channel will be encode/decode by the different material type.
  pub material_type_id: Attachment,
  pub channel_a: Attachment,
  pub channel_b: Attachment,
  pub channel_c: Attachment,
}

impl FrameGeneralMaterialBuffer {
  pub fn new(cx: &mut FrameCtx) -> Self {
    Self {
      material_type_id: attachment().format(TextureFormat::R8Uint).request(cx),
      channel_a: attachment()
        .format(TextureFormat::Rgba8UnormSrgb)
        .request(cx),
      channel_b: attachment()
        .format(TextureFormat::Rgba8UnormSrgb)
        .request(cx),
      channel_c: attachment().format(TextureFormat::Rg16Float).request(cx),
    }
  }
}

pub struct FrameGeneralMaterialBufferShaderInstance {
  pub material_type_id: HandleNode<ShaderTexture2D>,
  pub channel_a: HandleNode<ShaderTexture2D>,
  pub channel_b: HandleNode<ShaderTexture2D>,
  pub channel_c: HandleNode<ShaderTexture2D>,
}

#[derive(Hash)]
pub struct FrameGeneralMaterialChannelIndices {
  pub material_type_id: usize,
  pub channel_a: usize,
  pub channel_b: usize,
  pub channel_c: usize,
}

pub struct FrameGeneralMaterialBufferEncoder {
  pub indices: FrameGeneralMaterialChannelIndices,
  pub materials: DeferLightingMaterialRegistry,
}

pub struct DeferLightingMaterialRegistry {
  pub material_impl_ids: Vec<TypeId>,
  pub encoders:
    Vec<Box<dyn Fn(&mut ShaderFragmentBuilderView, &FrameGeneralMaterialChannelIndices)>>,
  pub decoders:
    Vec<Box<dyn Fn(&FrameGeneralMaterialBufferShaderInstance) -> Box<dyn LightableSurfaceShading>>>,
}

pub trait DeferLightingMaterialBufferReadWrite: 'static {
  fn encode(builder: &mut ShaderFragmentBuilderView, indices: &FrameGeneralMaterialChannelIndices);
  fn decode(
    instance: &FrameGeneralMaterialBufferShaderInstance,
  ) -> Box<dyn LightableSurfaceShading>;
}

impl DeferLightingMaterialRegistry {
  pub fn register_material_impl<M: DeferLightingMaterialBufferReadWrite>(&mut self) {
    self.material_impl_ids.push(TypeId::of::<M>());
    self.encoders.push(Box::new(M::encode));
    self.decoders.push(Box::new(M::decode));
  }
}

impl ShaderHashProvider for FrameGeneralMaterialBufferEncoder {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.indices.hash(hasher);
    self.materials.material_impl_ids.hash(hasher);
  }
}

impl GraphicsShaderProvider for FrameGeneralMaterialBufferEncoder {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      for m in &self.materials.encoders {
        m(builder, &self.indices);
      }
    })
  }
}
