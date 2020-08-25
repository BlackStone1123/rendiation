use crate::{
  AnyType, ShaderGraphAttributeNodeType, ShaderGraphBindGroupBuilder,
  ShaderGraphBindGroupItemProvider, ShaderGraphConstableNodeType, ShaderGraphNodeHandle,
  ShaderGraphNodeType,
};
use rendiation_math::*;
use rendiation_ral::ShaderStage;

impl ShaderGraphNodeType for AnyType {
  fn to_glsl_type() -> &'static str {
    unreachable!("Node can't newed with type AnyType")
  }
}

impl ShaderGraphNodeType for f32 {
  fn to_glsl_type() -> &'static str {
    "float"
  }
}
impl ShaderGraphAttributeNodeType for f32 {}
impl ShaderGraphConstableNodeType for f32 {
  fn const_to_glsl(&self) -> String {
    let mut result = format!("{}", self);
    if result.contains(".") {
      result
    } else {
      result.push_str(".0");
      result
    }
  }
}

impl ShaderGraphNodeType for Vec2<f32> {
  fn to_glsl_type() -> &'static str {
    "vec2"
  }
}
impl ShaderGraphAttributeNodeType for Vec2<f32> {}

impl ShaderGraphNodeType for Vec3<f32> {
  fn to_glsl_type() -> &'static str {
    "vec3"
  }
}
impl ShaderGraphAttributeNodeType for Vec3<f32> {}

impl ShaderGraphNodeType for Vec4<f32> {
  fn to_glsl_type() -> &'static str {
    "vec4"
  }
}
impl ShaderGraphAttributeNodeType for Vec4<f32> {}

impl ShaderGraphNodeType for Mat4<f32> {
  fn to_glsl_type() -> &'static str {
    "mat4"
  }
}

#[derive(Copy, Clone)]
pub struct ShaderGraphSampler;

impl ShaderGraphNodeType for ShaderGraphSampler {
  fn to_glsl_type() -> &'static str {
    "sampler"
  }
}

impl ShaderGraphBindGroupItemProvider for ShaderGraphSampler {
  type ShaderGraphBindGroupItemInstance = ShaderGraphNodeHandle<ShaderGraphSampler>;

  fn create_instance<'a>(
    name: &'static str,
    bindgroup_builder: &mut ShaderGraphBindGroupBuilder<'a>,
    stage: ShaderStage,
  ) -> Self::ShaderGraphBindGroupItemInstance {
    let node = bindgroup_builder.create_uniform_node::<ShaderGraphSampler>(name);
    bindgroup_builder.add_none_ubo(unsafe { node.handle.cast_type().into() }, stage);
    node
  }
}

#[derive(Copy, Clone)]
pub struct ShaderGraphTexture;

impl ShaderGraphNodeType for ShaderGraphTexture {
  fn to_glsl_type() -> &'static str {
    "texture2D"
  }
}

impl ShaderGraphNodeHandle<ShaderGraphTexture> {
  pub fn sample(
    _sampler: ShaderGraphNodeHandle<ShaderGraphSampler>,
    _position: ShaderGraphNodeHandle<Vec2<f32>>,
  ) {
    todo!()
  }
}

impl ShaderGraphBindGroupItemProvider for ShaderGraphTexture {
  type ShaderGraphBindGroupItemInstance = ShaderGraphNodeHandle<ShaderGraphTexture>;

  fn create_instance<'a>(
    name: &'static str,
    bindgroup_builder: &mut ShaderGraphBindGroupBuilder<'a>,
    stage: ShaderStage,
  ) -> Self::ShaderGraphBindGroupItemInstance {
    let node = bindgroup_builder.create_uniform_node::<ShaderGraphTexture>(name);
    bindgroup_builder.add_none_ubo(unsafe { node.handle.cast_type().into() }, stage);
    node
  }
}
