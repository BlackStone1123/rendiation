use std::{
  marker::PhantomData,
  task::{Context, Poll},
};

use reactive::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod storage;
pub use storage::*;
mod uniform_group;
pub use uniform_group::*;
mod uniform_array;
pub use uniform_array::*;
mod cube_map;
pub use cube_map::*;
