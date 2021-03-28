use crate::{
  concentric_sample_disk, rand, Diffuse, Intersection, Material, NormalizedVec3, PhysicalDiffuse,
  Vec3, INV_PI, PI,
};

use rendiation_algebra::IntoNormalizedVector;
use rendiation_algebra::{InnerProductSpace, Vec2};

pub struct Lambertian;
impl Material for Diffuse<Lambertian> {
  fn bsdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> Vec3 {
    self.albedo() / Vec3::splat(PI)
  }

  fn sample_light_dir(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> NormalizedVec3 {
    // Simple cosine-sampling using Malley's method
    let sample = concentric_sample_disk(Vec2::new(rand(), rand()));
    let x = sample.x;
    let y = sample.y;
    let z = (1.0 - x * x - y * y).sqrt();
    (Vec3::new(x, y, z) * intersection.geometric_normal.local_to_world()).into_normalized()
  }

  fn pdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> f32 {
    light_dir.dot(intersection.geometric_normal).max(0.0) * INV_PI
  }
}

impl PhysicalDiffuse for Diffuse<Lambertian> {
  fn albedo(&self) -> Vec3 {
    self.albedo
  }
}

pub struct OrenNayar {
  /// the standard deviation of the microfacet orientation angle
  /// in radians
  sigma: f32,
  albedo: Vec3,
  a: f32,
  b: f32,
}

impl OrenNayar {
  fn new(albedo: Vec3, sigma: f32) -> Self {
    let sigma2 = sigma * sigma;
    let a = 1. - (sigma2 / (2. * (sigma2 + 0.33)));
    let b = 0.45 * sigma2 / (sigma2 + 0.09);
    Self {
      sigma,
      albedo,
      a,
      b,
    }
  }
}

impl Material for OrenNayar {
  fn bsdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> Vec3 {
    todo!()
    // let sin_theta_i = sin_theta(wi);
    // let sin_theta_o = sin_theta(wo);
    // // compute cosine term of Oren-Nayar model
    // let max_cos = if sin_theta_i > 1.0e-4 && sin_theta_o > 1.0e-4 {
    //   let sin_phi_i = sin_phi(wi);
    //   let cos_phi_i = cos_phi(wi);
    //   let sin_phi_o = sin_phi(wo);
    //   let cos_phi_o = cos_phi(wo);
    //   let d_cos = cos_phi_i * cos_phi_o + sin_phi_i * sin_phi_o;
    //   d_cos.max(0.0 as Float)
    // } else {
    //   0.0 as Float
    // };
    // // compute sine and tangent terms of Oren-Nayar model
    // let sin_alpha;
    // let tan_beta = if abs_cos_theta(wi) > abs_cos_theta(wo) {
    //   sin_alpha = sin_theta_o;
    //   sin_theta_i / abs_cos_theta(wi)
    // } else {
    //   sin_alpha = sin_theta_i;
    //   sin_theta_o / abs_cos_theta(wo)
    // };
    // self.albedo * INV_PI * (self.a + self.b * max_cos * sin_alpha * tan_beta)
  }

  fn sample_light_dir(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> NormalizedVec3 {
    todo!()
  }

  fn pdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> f32 {
    todo!()
  }
}
