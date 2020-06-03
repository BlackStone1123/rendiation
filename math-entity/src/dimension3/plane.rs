use crate::{MultiDimensionalLine, Triangle};
use rendiation_math::*;

// we cant use type alias for trait bound in stable: https://github.com/rust-lang/rust/issues/52662
pub type Plane = MultiDimensionalLine<f32, Vec3<f32>>;

impl Plane {
  pub fn distance_to_point(&self, point: Vec3<f32>) -> f32 {
    self.normal.dot(point) + self.constant
  }

  pub fn project_point(&self, point: Vec3<f32>) -> Vec3<f32> {
    self.normal * (-self.distance_to_point(point)) + point
  }

  pub fn set_components(&mut self, x: f32, y: f32, z: f32, w: f32) -> &mut Self {
    self.normal.set(x, y, z);
    self.constant = w;
    self
  }

  pub fn normalize(&mut self) -> &mut Self {
    let inverse_normal_length = 1.0 / self.normal.length();
    self.normal *= inverse_normal_length;
    self.constant *= inverse_normal_length;
    self
  }
}

impl From<Triangle> for Plane {
  fn from(face: Triangle) -> Plane {
    let v1 = face.b - face.a;
    let v2 = face.c - face.a;
    let normal = v1.cross(v2).normalize();
    let constant = normal.dot(face.a);
    Plane::new(normal, constant)
  }
}
