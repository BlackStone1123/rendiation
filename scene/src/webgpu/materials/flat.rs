use std::rc::Rc;

use rendiation_algebra::Vec4;
use rendiation_renderable_mesh::vertex::Vertex;
use rendiation_webgpu::*;

use crate::*;

impl MaterialMeshLayoutRequire for FlatMaterial {
  type VertexInput = Vec<Vertex>;
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderUniform)]
pub struct FlatMaterialUniform {
  pub color: Vec4<f32>,
}

pub struct MaterialUniform<T> {
  pub inner: UniformBuffer<T>,
}

impl<T: ShaderGraphNodeType> SemanticShaderUniform for MaterialUniform<T> {
  const TYPE: SemanticBinding = SemanticBinding::Material;
  type Node = T;
}

impl<T> BindProvider for MaterialUniform<T> {
  fn as_bindable(&self) -> wgpu::BindingResource {
    self.inner.as_bindable()
  }

  fn add_bind_record(&self, record: BindGroupCacheInvalidation) {
    todo!()
  }
}

impl SemanticShaderUniform for FlatMaterialUniform {
  const TYPE: SemanticBinding = SemanticBinding::Material;
  type Node = Self;
}

impl ShaderBindingProvider for FlatMaterialGPU {
  fn setup_binding<'a>(&'a self, builder: &mut BindGroupBuilder<'a>) {
    builder.setup_uniform(&self.uniform);
  }
}

impl ShaderGraphProvider for FlatMaterialGPU {
  fn build_fragment(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let uniform = builder.register_uniform_by(&self.uniform).expand();

    builder.set_fragment_out(0, uniform.color);
    Ok(())
  }
}

pub struct FlatMaterialGPU {
  uniform: MaterialUniform<FlatMaterialUniform>,
}

impl WebGPUMaterial for FlatMaterial {
  type GPU = FlatMaterialGPU;

  fn create_gpu(&self, res: &mut GPUResourceSubCache) -> Self::GPU {
    FlatMaterialGPU {
      uniform: res.uniforms.get(self.uniform),
    }
  }

  fn is_keep_mesh_shape(&self) -> bool {
    true
  }

  fn is_transparent(&self) -> bool {
    false
  }
}
