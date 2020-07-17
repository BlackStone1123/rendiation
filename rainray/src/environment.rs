use crate::math::Vec3;
use rendiation_math_entity::Ray3;

pub struct SolidEnvironment {
  pub intensity: Vec3,
}

pub trait Environment: Sync {
  fn sample(&self, ray: &Ray3) -> Vec3;
}

impl Environment for SolidEnvironment {
  fn sample(&self, _ray: &Ray3) -> Vec3 {
    self.intensity
  }
}

pub struct GradientEnvironment {
  pub top_intensity: Vec3,
  pub bottom_intensity: Vec3,
}

impl Environment for GradientEnvironment {
  fn sample(&self, ray: &Ray3) -> Vec3 {
    let t = ray.direction.y / 2.0 + 1.;
    Vec3::new(
      lerp(t, self.bottom_intensity.x, self.top_intensity.x),
      lerp(t, self.bottom_intensity.y, self.top_intensity.y),
      lerp(t, self.bottom_intensity.z, self.top_intensity.z),
    )
  }
}

fn lerp(t: f32, min: f32, max: f32) -> f32 {
  (1. - t) * min + t * max
}
