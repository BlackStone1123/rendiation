pub mod pass;
pub use pass::*;

pub mod attachment;
pub use attachment::*;

use crate::*;

pub struct FrameCtx<'a> {
  pub gpu: &'a GPU,
  pool: &'a ResourcePool,
  msaa_sample_count: u32,
  frame_size: Size,
  encoder: GPUCommandEncoder,
  pub resources: &'a mut GPUResourceCache,
}

impl<'a> FrameCtx<'a> {
  pub fn new(
    gpu: &'a GPU,
    frame_size: Size,
    pool: &'a ResourcePool,
    resources: &'a mut GPUResourceCache,
  ) -> Self {
    let msaa_sample_count = 4;

    let encoder = gpu.create_encoder();

    Self {
      pool,
      frame_size,
      resources,
      msaa_sample_count,
      encoder,
      gpu,
    }
  }

  pub fn submit(self) {
    self.gpu.submit_encoder(self.encoder)
  }

  pub fn notify_output_resized(&self) {
    self.pool.inner.borrow_mut().clear();
  }

  pub fn multisampled_attachment(&self) -> AttachmentDescriptor {
    AttachmentDescriptor {
      format: webgpu::TextureFormat::Rgba8Unorm,
      sample_count: self.msaa_sample_count,
      sizer: default_sizer(),
    }
  }

  pub fn multisampled_depth_attachment(&self) -> AttachmentDescriptor {
    AttachmentDescriptor {
      format: webgpu::TextureFormat::Depth24PlusStencil8,
      sample_count: self.msaa_sample_count,
      sizer: default_sizer(),
    }
  }
}
