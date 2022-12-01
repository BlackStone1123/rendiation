use crate::*;

macro_rules! sg_node_impl {
  ($ty: ty, $ty_value: expr) => {
    impl ShaderGraphNodeType for $ty {
      const TYPE: ShaderValueType = $ty_value;
    }
  };
}

sg_node_impl!(AnyType, ShaderValueType::Never);
sg_node_impl!(ShaderSampler, ShaderValueType::Sampler);
sg_node_impl!(ShaderCompareSampler, ShaderValueType::CompareSampler);

// Impl Notes:
//
// impl<T: PrimitiveShaderGraphNodeType> ShaderGraphNodeType for T {
//   const TYPE: ShaderValueType =
//     ShaderValueType::Fixed(ShaderStructMemberValueType::Primitive(T::PRIMITIVE_TYPE));
// }
// impl<T: PrimitiveShaderGraphNodeType> ShaderStructMemberValueNodeType for T {
//   const TYPE: ShaderStructMemberValueType =
//     ShaderStructMemberValueType::Primitive(T::PRIMITIVE_TYPE);
// }
//
// We can not use above auto impl but the macro because rust not support trait associate const specialization

/// Impl note: why we not use the follow code instead of macro?
macro_rules! primitive_ty {
  ($ty: ty, $primitive_ty_value: expr, $to_primitive: expr) => {
    sg_node_impl!(
      $ty,
      ShaderValueType::Fixed(ShaderStructMemberValueType::Primitive($primitive_ty_value))
    );

    impl ShaderStructMemberValueNodeType for $ty {
      const MEMBER_TYPE: ShaderStructMemberValueType =
        ShaderStructMemberValueType::Primitive($primitive_ty_value);
    }

    impl PrimitiveShaderGraphNodeType for $ty {
      const PRIMITIVE_TYPE: PrimitiveShaderValueType = $primitive_ty_value;
      fn to_primitive(&self) -> PrimitiveShaderValue {
        $to_primitive(*self)
      }
    }
  };
}

// we group them together just to skip rustfmt entirely
#[rustfmt::skip]
mod impls {
  use crate::*;
  primitive_ty!(bool, PrimitiveShaderValueType::Bool,  PrimitiveShaderValue::Bool);
  primitive_ty!(u32, PrimitiveShaderValueType::Uint32,  PrimitiveShaderValue::Uint32);
  primitive_ty!(i32, PrimitiveShaderValueType::Int32,  PrimitiveShaderValue::Int32);
  primitive_ty!(f32, PrimitiveShaderValueType::Float32,  PrimitiveShaderValue::Float32);
  primitive_ty!(Vec2<f32>, PrimitiveShaderValueType::Vec2Float32,  PrimitiveShaderValue::Vec2Float32);
  primitive_ty!(Vec3<f32>, PrimitiveShaderValueType::Vec3Float32,  PrimitiveShaderValue::Vec3Float32);
  primitive_ty!(Vec4<f32>, PrimitiveShaderValueType::Vec4Float32,  PrimitiveShaderValue::Vec4Float32);
  primitive_ty!(Vec2<u32>, PrimitiveShaderValueType::Vec2Uint32,  PrimitiveShaderValue::Vec2Uint32);
  primitive_ty!(Vec3<u32>, PrimitiveShaderValueType::Vec3Uint32,  PrimitiveShaderValue::Vec3Uint32);
  primitive_ty!(Vec4<u32>, PrimitiveShaderValueType::Vec4Uint32,  PrimitiveShaderValue::Vec4Uint32);
  primitive_ty!(Mat2<f32>, PrimitiveShaderValueType::Mat2Float32,  PrimitiveShaderValue::Mat2Float32);
  primitive_ty!(Mat3<f32>, PrimitiveShaderValueType::Mat3Float32,  PrimitiveShaderValue::Mat3Float32);
  primitive_ty!(Mat4<f32>, PrimitiveShaderValueType::Mat4Float32,  PrimitiveShaderValue::Mat4Float32);
}

macro_rules! vertex_input_node_impl {
  ($ty: ty, $format: expr) => {
    impl VertexInShaderGraphNodeType for $ty {
      fn to_vertex_format() -> VertexFormat {
        $format
      }
    }
  };
}
vertex_input_node_impl!(f32, VertexFormat::Float32);
vertex_input_node_impl!(Vec2<f32>, VertexFormat::Float32x2);
vertex_input_node_impl!(Vec3<f32>, VertexFormat::Float32x3);
vertex_input_node_impl!(Vec4<f32>, VertexFormat::Float32x4);

// these impl not use macro because not helping
impl ShaderGraphNodeType for ShaderTexture2D {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::D2,
    sample_type: TextureSampleType::Float { filterable: true },
  };
}
impl ShaderGraphNodeType for ShaderTextureCube {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::Cube,
    sample_type: TextureSampleType::Float { filterable: true },
  };
}
impl ShaderGraphNodeType for ShaderTexture1D {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::D1,
    sample_type: TextureSampleType::Float { filterable: true },
  };
}
impl ShaderGraphNodeType for ShaderTexture3D {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::D3,
    sample_type: TextureSampleType::Float { filterable: true },
  };
}
impl ShaderGraphNodeType for ShaderTexture2DArray {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::D2Array,
    sample_type: TextureSampleType::Float { filterable: true },
  };
}
impl ShaderGraphNodeType for ShaderTextureCubeArray {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::CubeArray,
    sample_type: TextureSampleType::Float { filterable: true },
  };
}

impl ShaderGraphNodeType for ShaderDepthTexture2D {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::D2,
    sample_type: TextureSampleType::Depth,
  };
}
impl ShaderGraphNodeType for ShaderDepthTexture2DArray {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::D2Array,
    sample_type: TextureSampleType::Depth,
  };
}
impl ShaderGraphNodeType for ShaderDepthTextureCube {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::Cube,
    sample_type: TextureSampleType::Depth,
  };
}
impl ShaderGraphNodeType for ShaderDepthTextureCubeArray {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::CubeArray,
    sample_type: TextureSampleType::Depth,
  };
}

/// https://www.w3.org/TR/WGSL/#texturesample
pub trait SingleSampleTarget {
  type Input;
  type Sampler;
  type Output: PrimitiveShaderGraphNodeType;
}

impl SingleSampleTarget for ShaderTexture1D {
  type Input = f32;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

impl SingleSampleTarget for ShaderTexture2D {
  type Input = Vec2<f32>;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

impl SingleSampleTarget for ShaderDepthTexture2D {
  type Input = Vec2<f32>;
  type Sampler = ShaderSampler;
  type Output = f32;
}

impl SingleSampleTarget for ShaderTexture3D {
  type Input = Vec3<f32>;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

impl SingleSampleTarget for ShaderTextureCube {
  type Input = Vec3<f32>;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

impl SingleSampleTarget for ShaderDepthTextureCube {
  type Input = Vec3<f32>;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

pub trait ArraySampleTarget {
  type Input;
  type Sampler;
  type Output: PrimitiveShaderGraphNodeType;
}

impl ArraySampleTarget for ShaderTexture2DArray {
  type Input = Vec2<f32>;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

impl ArraySampleTarget for ShaderTextureCubeArray {
  type Input = Vec2<f32>;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

impl ArraySampleTarget for ShaderDepthTexture2DArray {
  type Input = Vec2<f32>;
  type Sampler = ShaderSampler;
  type Output = f32;
}

impl ArraySampleTarget for ShaderDepthTextureCubeArray {
  type Input = Vec2<f32>;
  type Sampler = ShaderSampler;
  type Output = f32;
}

impl<T: SingleSampleTarget> Node<T> {
  pub fn sample(&self, sampler: Node<T::Sampler>, position: Node<T::Input>) -> Node<T::Output> {
    ShaderGraphNodeExpr::TextureSampling {
      texture: self.handle(),
      sampler: sampler.handle(),
      position: position.handle(),
      index: None,
      level: None,
    }
    .insert_graph()
  }

  pub fn sample_level(
    &self,
    sampler: Node<T::Sampler>,
    position: Node<T::Input>,
    level: Node<f32>,
  ) -> Node<T::Output> {
    ShaderGraphNodeExpr::TextureSampling {
      texture: self.handle(),
      sampler: sampler.handle(),
      position: position.handle(),
      index: None,
      level: level.handle().into(),
    }
    .insert_graph()
  }
}

pub trait ShaderArrayTextureSampleIndexType: ShaderGraphNodeType {}
impl ShaderArrayTextureSampleIndexType for u32 {}
impl ShaderArrayTextureSampleIndexType for i32 {}

impl<T: ArraySampleTarget> Node<T> {
  pub fn sample_index(
    &self,
    sampler: Node<T::Sampler>,
    position: Node<T::Input>,
    index: Node<impl ShaderArrayTextureSampleIndexType>,
  ) -> Node<T::Output> {
    ShaderGraphNodeExpr::TextureSampling {
      texture: self.handle(),
      sampler: sampler.handle(),
      position: position.handle(),
      index: index.handle().into(),
      level: None,
    }
    .insert_graph()
  }

  pub fn sample_index_level(
    &self,
    sampler: Node<T::Sampler>,
    position: Node<T::Input>,
    index: Node<impl ShaderArrayTextureSampleIndexType>,
    level: Node<f32>,
  ) -> Node<T::Output> {
    ShaderGraphNodeExpr::TextureSampling {
      texture: self.handle(),
      sampler: sampler.handle(),
      position: position.handle(),
      index: index.handle().into(),
      level: level.handle().into(),
    }
    .insert_graph()
  }
}
