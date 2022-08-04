use crate::*;

pub struct ParametricRangeMapping<T> {
  pub inner: T,
  pub start: f32,
  pub end: f32,
}
impl<T: ParametricCurve3D> ParametricCurve3D for ParametricRangeMapping<T> {
  fn position(&self, position: f32) -> Vec3<f32> {
    let mapped = self.start.lerp(self.end, position);
    self.inner.position(mapped)
  }
  fn normal(&self, position: f32) -> Vec3<f32> {
    let mapped = self.start.lerp(self.end, position);
    self.inner.normal(mapped)
  }
}
pub trait IntoParametricRangeMapping: ParametricCurve3D + Sized {
  fn map_range(self, range: Range<f32>) -> ParametricRangeMapping<Self> {
    ParametricRangeMapping {
      inner: self,
      start: range.start,
      end: range.end,
    }
  }
}
impl<T: ParametricCurve3D + Sized> IntoParametricRangeMapping for T {}

pub struct Embed2DCurveTo3DSurface<S, T> {
  pub curve: S,
  pub surface: T,
}
pub trait IntoEmbed2DCurveTo3DSurface: ParametricCurve2D + Sized {
  fn embed_to_surface<T>(self, surface: T) -> Embed2DCurveTo3DSurface<Self, T>
  where
    T: ParametricSurface,
  {
    Embed2DCurveTo3DSurface {
      curve: self,
      surface,
    }
  }
}
impl<S> IntoEmbed2DCurveTo3DSurface for S where S: ParametricCurve2D + Sized {}
impl<S, T> ParametricCurve3D for Embed2DCurveTo3DSurface<S, T>
where
  S: ParametricCurve2D,
  T: ParametricSurface,
{
  fn position(&self, position: f32) -> Vec3<f32> {
    let curve_space = self.curve.position(position);
    self.surface.position(curve_space)
  }

  fn normal(&self, position: f32) -> Vec3<f32> {
    let curve_space = self.curve.position(position);
    self.surface.normal(curve_space)
  }
}

pub struct FixedSweepSurface<T, P> {
  pub cross_section_outline: T,
  pub path: P,
}
pub trait IntoFixedFixedSweepSurfaceFromPath: ParametricCurve2D + Sized {
  fn fix_swap_by_path<P>(self, path: P) -> FixedSweepSurface<Self, P>
  where
    P: ParametricCurve3D,
  {
    FixedSweepSurface {
      cross_section_outline: self,
      path,
    }
  }
}
impl<T> IntoFixedFixedSweepSurfaceFromPath for T where T: ParametricCurve2D + Sized {}
pub trait IntoFixedFixedSweepSurfaceFromCrossSection: ParametricCurve3D + Sized {
  fn make_tube_by<T>(self, cross_section_outline: T) -> FixedSweepSurface<T, Self>
  where
    T: ParametricCurve2D,
  {
    FixedSweepSurface {
      cross_section_outline,
      path: self,
    }
  }
}
impl<T> IntoFixedFixedSweepSurfaceFromCrossSection for T where T: ParametricCurve3D + Sized {}
impl<T, P> ParametricSurface for FixedSweepSurface<T, P>
where
  T: ParametricCurve2D,
  P: ParametricCurve3D,
{
  fn position(&self, position: Vec2<f32>) -> Vec3<f32> {
    let path_dimension = position.x;
    let cross_section_dimension = position.y;
    let cross_section_point = self.cross_section_outline.position(cross_section_dimension);
    let cross_section_point = Vec3::new(cross_section_point.x, cross_section_point.y, 0.);

    let cross_section_origin = self.path.position(path_dimension);
    let cross_section_normal = self.path.normal(path_dimension);
    let cross_section_tangent = self.path.tangent(path_dimension);

    // should be cheaper?
    Mat4::from_orth_basis_and_position(
      cross_section_tangent,
      cross_section_normal,
      cross_section_origin,
    ) * cross_section_point
  }
}

pub struct Transformed2D<T> {
  curve: T,
  mat: Mat3<f32>,
  normal_mat: Mat2<f32>,
}
impl<T: ParametricCurve2D> ParametricCurve2D for Transformed2D<T> {
  fn position(&self, position: f32) -> Vec2<f32> {
    self.mat * self.curve.position(position)
  }

  fn tangent(&self, position: f32) -> Vec2<f32> {
    self.normal_mat * self.curve.tangent(position)
  }

  fn normal(&self, position: f32) -> Vec2<f32> {
    self.normal_mat * self.curve.normal(position)
  }
}

pub trait IntoTransformed2D: ParametricCurve2D + Sized {
  fn transform_by(self, mat: Mat3<f32>) -> Transformed2D<Self> {
    Transformed2D {
      curve: self,
      mat,
      normal_mat: mat.to_normal_matrix(),
    }
  }
}
impl<T> IntoTransformed2D for T where T: ParametricCurve2D + Sized {}
