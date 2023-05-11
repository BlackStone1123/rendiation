use bytemuck::*;

use crate::*;

/// GLSL's `bool` type.
///
/// Boolean values in GLSL are 32 bits, in contrast with Rust's 8 bit bools.
#[derive(Clone, Copy, Eq, PartialEq, Zeroable, Pod, Default, Hash)]
#[repr(transparent)]
pub struct Bool(u32);

impl From<bool> for Bool {
  fn from(v: bool) -> Self {
    Self(v as u32)
  }
}

impl From<Bool> for bool {
  fn from(v: Bool) -> Self {
    v.0 != 0
  }
}

use core::fmt::{Debug, Formatter};

impl Debug for Bool {
  fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
    write!(f, "Bool({:?})", bool::from(*self))
  }
}

#[repr(C)]
#[rustfmt::skip]
#[derive(Clone, Copy, Zeroable, Pod, PartialEq, Default)]
pub struct Shader140Mat3 {
  pub a1: f32, pub a2: f32, pub a3: f32, _pad1: f32,
  pub b1: f32, pub b2: f32, pub b3: f32, _pad2: f32,
  pub c1: f32, pub c2: f32, pub c3: f32, _pad3: f32,
}

impl From<Mat3<f32>> for Shader140Mat3 {
  #[rustfmt::skip]
  fn from(v: Mat3<f32>) -> Self {
    Self {
      a1: v.a1, a2: v.a2, a3: v.a3,
      b1: v.b1, b2: v.b2, b3: v.b3,
      c1: v.c1, c2: v.c2, c3: v.c3,
      ..Default::default()
    }
  }
}

impl From<Shader140Mat3> for Mat3<f32> {
  #[rustfmt::skip]
  fn from(v: Shader140Mat3) -> Self {
    Self {
      a1: v.a1, a2: v.a2, a3: v.a3,
      b1: v.b1, b2: v.b2, b3: v.b3,
      c1: v.c1, c2: v.c2, c3: v.c3,
    }
  }
}

#[repr(C)]
#[rustfmt::skip]
#[derive(Clone, Copy, Zeroable, Pod, PartialEq, Default)]
pub struct Shader140Mat2 {
  pub a1:f32, pub a2:f32, _pad1: [f32; 2],
  pub b1:f32, pub b2:f32, _pad2: [f32; 2],
}

impl From<Mat2<f32>> for Shader140Mat2 {
  #[rustfmt::skip]
  fn from(v: Mat2<f32>) -> Self {
    Self {
      a1: v.a1, a2: v.a2,
      b1: v.b1, b2: v.b2,
      ..Default::default()
    }
  }
}

impl From<Shader140Mat2> for Mat2<f32> {
  #[rustfmt::skip]
  fn from(v: Shader140Mat2) -> Self {
    Self {
      a1: v.a1, a2: v.a2,
      b1: v.b1, b2: v.b2,
    }
  }
}
