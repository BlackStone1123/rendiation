use crate::primitive::*;
use crate::renderer::pipeline::*;
use crate::{vertex::Vertex, VertexBufferDescriptor};
use core::marker::PhantomData;

/// A indexed geometry that use vertex as primitive;
pub struct StandardGeometry<T = TriangleList, V = Vertex>
where
  V: VertexProvider,
  T: PrimitiveTopology,
{
  pub data: Vec<V>,
  pub index: Vec<u16>,
  _phantom: PhantomData<T>,
}

impl From<(Vec<Vertex>, Vec<u16>)> for StandardGeometry {
  fn from(item: (Vec<Vertex>, Vec<u16>)) -> Self {
    StandardGeometry::new(item.0, item.1)
  }
}

impl<T: PrimitiveTopology> StandardGeometry<T> {
  pub fn new(v: Vec<Vertex>, index: Vec<u16>) -> Self {
    Self {
      data: v,
      index,
      _phantom: PhantomData,
    }
  }

  pub fn primitive_iter<'a>(&'a self) -> IndexedPrimitiveIter<'a, T::Primitive> {
    IndexedPrimitiveIter::new(&self.index, &self.data)
  }

  pub fn get_primitive_count(&self) -> u32 {
    self.index.len() as u32 / T::STRIDE as u32
  }

  pub fn get_full_count(&self) -> u32 {
    self.index.len() as u32
  }
}

use lazy_static::lazy_static;
lazy_static! {
  static ref VERTEX_BUFFERS: Vec<VertexBufferDescriptor<'static>> =
    { vec![Vertex::get_buffer_layout_descriptor()] };
}

impl<'a, T: PrimitiveTopology> GeometryProvider for StandardGeometry<T> {
  fn get_geometry_vertex_state_descriptor() -> wgpu::VertexStateDescriptor<'static> {
    wgpu::VertexStateDescriptor {
      index_format: wgpu::IndexFormat::Uint16,
      vertex_buffers: &VERTEX_BUFFERS,
    }
  }

  fn get_primitive_topology() -> wgpu::PrimitiveTopology {
    T::WGPU_ENUM
  }
}
