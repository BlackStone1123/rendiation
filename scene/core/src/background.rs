use crate::*;

pub enum SceneBackGround {
  Solid(SolidBackground),
  Env(EnvMapBackground),
  Foreign(Box<dyn ForeignImplemented>),
}

pub struct SolidBackground {
  pub intensity: Vec3<f32>,
}

impl Default for SolidBackground {
  fn default() -> Self {
    Self {
      intensity: Vec3::new(0.6, 0.6, 0.6),
    }
  }
}

impl SolidBackground {
  pub fn black() -> Self {
    Self {
      intensity: Vec3::splat(0.0),
    }
  }
}

pub struct EnvMapBackground {
  pub texture: SceneTextureCube,
}
