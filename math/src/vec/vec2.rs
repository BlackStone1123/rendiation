use crate::*;
use std::fmt;
use std::fmt::Debug;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Vec2<T> {
  pub x: T,
  pub y: T,
}

unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Vec2<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Vec2<T> {}

impl<T: Scalar> VectorDimension<2> for Vec2<T> {}
impl<T: Scalar> VectorImpl for Vec2<T> {}
impl<T: Scalar> Vector<T> for Vec2<T> {
  #[inline]
  fn dot(&self, b: Self) -> T {
    self.x * b.x + self.y * b.y
  }

  #[inline]
  fn cross(&self, b: Self) -> Self {
    Self {
      x: self.y * b.x - self.x * b.y,
      y: self.x * b.y - self.y * b.x,
    }
  }
}

impl<T> Vec2<T>
where
  T: Scalar,
{
  #[inline]
  pub fn rotate(&self, anchor: Self, radians: T) -> Self {
    let v = *self - anchor;
    let x = v.x;
    let y = v.y;
    let c = radians.cos();
    let s = radians.sin();
    Self {
      x: x * c - y * s,
      y: x * s + y * c,
    }
  }
}

impl<T> Math for Vec2<T>
where
  T: Copy + Math,
{
  #[inline]
  fn abs(self) -> Self {
    let mx = self.x.abs();
    let my = self.y.abs();
    Self { x: mx, y: my }
  }

  #[inline]
  fn recip(self) -> Self {
    let mx = self.x.recip();
    let my = self.y.recip();
    Self { x: mx, y: my }
  }

  #[inline]
  fn sqrt(self) -> Self {
    let mx = self.x.sqrt();
    let my = self.y.sqrt();
    Self { x: mx, y: my }
  }

  #[inline]
  fn rsqrt(self) -> Self {
    let mx = self.x.rsqrt();
    let my = self.y.rsqrt();
    Self { x: mx, y: my }
  }

  #[inline]
  fn sin(self) -> Self {
    let mx = self.x.sin();
    let my = self.y.sin();
    Self { x: mx, y: my }
  }

  #[inline]
  fn cos(self) -> Self {
    let mx = self.x.cos();
    let my = self.y.cos();
    Self { x: mx, y: my }
  }

  #[inline]
  fn tan(self) -> Self {
    let mx = self.x.tan();
    let my = self.y.tan();
    Self { x: mx, y: my }
  }

  #[inline]
  fn sincos(self) -> (Self, Self) {
    let mx = self.x.sincos();
    let my = self.y.sincos();
    (Self { x: mx.0, y: my.0 }, Self { x: mx.1, y: my.1 })
  }

  #[inline]
  fn acos(self) -> Self {
    let mx = self.x.acos();
    let my = self.y.acos();
    Self { x: mx, y: my }
  }

  #[inline]
  fn asin(self) -> Self {
    let mx = self.x.asin();
    let my = self.y.asin();
    Self { x: mx, y: my }
  }

  #[inline]
  fn atan(self) -> Self {
    let mx = self.x.atan();
    let my = self.y.atan();
    Self { x: mx, y: my }
  }

  #[inline]
  fn exp(self) -> Self {
    let mx = self.x.exp();
    let my = self.y.exp();
    Self { x: mx, y: my }
  }

  #[inline]
  fn exp2(self) -> Self {
    let mx = self.x.exp2();
    let my = self.y.exp2();
    Self { x: mx, y: my }
  }

  #[inline]
  fn log(self, rhs: Self) -> Self {
    let mx = self.x.log(rhs.x);
    let my = self.y.log(rhs.y);
    Self { x: mx, y: my }
  }

  #[inline]
  fn log2(self) -> Self {
    let mx = self.x.log2();
    let my = self.y.log2();
    Self { x: mx, y: my }
  }

  #[inline]
  fn log10(self) -> Self {
    let mx = self.x.log10();
    let my = self.y.log10();
    Self { x: mx, y: my }
  }

  #[inline]
  fn to_radians(self) -> Self {
    let mx = self.x.to_radians();
    let my = self.y.to_radians();
    Self { x: mx, y: my }
  }

  #[inline]
  fn to_degrees(self) -> Self {
    let mx = self.x.to_degrees();
    let my = self.y.to_degrees();
    Self { x: mx, y: my }
  }

  #[inline]
  fn min(self, rhs: Self) -> Self {
    let mx = self.x.min(rhs.x);
    let my = self.y.min(rhs.y);
    Self { x: mx, y: my }
  }

  #[inline]
  fn max(self, rhs: Self) -> Self {
    let mx = self.x.max(rhs.x);
    let my = self.y.max(rhs.y);
    Self { x: mx, y: my }
  }

  #[inline]
  fn saturate(self) -> Self {
    let mx = self.x.saturate();
    let my = self.y.saturate();
    Self { x: mx, y: my }
  }

  #[inline]
  fn snorm2unorm(self) -> Self {
    let mx = self.x.snorm2unorm();
    let my = self.y.snorm2unorm();
    Self { x: mx, y: my }
  }

  #[inline]
  fn unorm2snorm(self) -> Self {
    let mx = self.x.unorm2snorm();
    let my = self.y.unorm2snorm();
    Self { x: mx, y: my }
  }

  #[inline]
  fn clamp(self, minval: Self, maxval: Self) -> Self {
    let mx = self.x.clamp(minval.x, maxval.x);
    let my = self.y.clamp(minval.y, maxval.y);
    Self { x: mx, y: my }
  }
}

impl<T> Zero for Vec2<T>
where
  T: Zero,
{
  #[inline(always)]
  fn zero() -> Self {
    Self {
      x: T::zero(),
      y: T::zero(),
    }
  }
}

impl<T> One for Vec2<T>
where
  T: One,
{
  #[inline(always)]
  fn one() -> Self {
    Self {
      x: T::one(),
      y: T::one(),
    }
  }
}

impl<T> fmt::Display for Vec2<T>
where
  T: Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "({:?}, {:?})", self.x, self.y)
  }
}
