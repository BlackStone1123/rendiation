use crate::WGPURenderer;

pub struct SwapChain {
  surface: wgpu::Surface,
  pub swap_chain: wgpu::SwapChain,
  pub swap_chain_descriptor: wgpu::SwapChainDescriptor,
  pub hidpi_factor: f32,
  pub size: (usize, usize),
}

impl SwapChain {
  pub fn new(
    surface: wgpu::Surface,
    size: (usize, usize),
    renderer: &WGPURenderer,
    hidpi_factor: f32,
  ) -> Self {
    let swap_chain_descriptor = wgpu::SwapChainDescriptor {
      usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
      format: renderer.swap_chain_format,
      width: size.0 as u32,
      height: size.1 as u32,
      present_mode: wgpu::PresentMode::Vsync,
    };
    let swap_chain = renderer.device.create_swap_chain(&surface, &swap_chain_descriptor);
    Self {
      surface,
      swap_chain_descriptor,
      swap_chain,
      hidpi_factor,
      size,
    }
  }

  pub fn resize(&mut self, size: (usize, usize), device: &wgpu::Device) {
    self.swap_chain_descriptor.width = size.0 as u32;
    self.swap_chain_descriptor.height = size.1 as u32;
    self.swap_chain =
      device.create_swap_chain(&self.surface, &self.swap_chain_descriptor);
    self.size = size;
  }

  pub fn request_output(&mut self) -> wgpu::SwapChainOutput {
    self.swap_chain.get_next_texture()
  }
}
