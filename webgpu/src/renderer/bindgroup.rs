use crate::renderer::buffer::WGPUBuffer;
use crate::{renderer::sampler::WGPUSampler, shader_stage_convert, WGPURenderer};
use std::ops::Range;

pub enum WGPUBinding<'a> {
  BindBuffer((&'a WGPUBuffer, Range<u64>)),
  BindTexture(&'a wgpu::TextureView),
  BindSampler(&'a WGPUSampler),
}

pub struct WGPUBindGroup {
  pub gpu_bindgroup: wgpu::BindGroup,
}

impl WGPUBindGroup {
  pub fn new(
    device: &wgpu::Device,
    bindings: &[WGPUBinding],
    layout: &wgpu::BindGroupLayout,
  ) -> Self {
    let wgpu_bindings: Vec<_> = bindings
      .iter()
      .enumerate()
      .map(|(i, binding)| {
        let resource = match binding {
          WGPUBinding::BindBuffer((buffer, range)) => {
            wgpu::BindingResource::Buffer(buffer.get_gpu_buffer().slice(*range))
          }
          WGPUBinding::BindTexture(texture) => wgpu::BindingResource::TextureView(&texture),
          WGPUBinding::BindSampler(sampler) => {
            wgpu::BindingResource::Sampler(sampler.get_gpu_sampler())
          }
        };
        wgpu::Binding {
          binding: i as u32,
          resource,
        }
      })
      .collect();

    let wgpu_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: None,
      layout,
      bindings: &wgpu_bindings,
    });

    Self {
      gpu_bindgroup: wgpu_bindgroup,
    }
  }
}

#[derive(Default)]
pub struct BindGroupBuilder<'a> {
  pub bindings: Vec<WGPUBinding<'a>>,
}

impl<'a> BindGroupBuilder<'a> {
  pub fn new() -> Self {
    Self {
      bindings: Vec::new(),
    }
  }

  pub fn push(mut self, b: WGPUBinding<'a>) -> Self {
    self.bindings.push(b);
    self
  }

  pub fn buffer(mut self, b: (&'a WGPUBuffer, Range<u64>)) -> Self {
    self.bindings.push(WGPUBinding::BindBuffer(b));
    self
  }

  pub fn texture(mut self, t: &'a wgpu::TextureView) -> Self {
    self.bindings.push(WGPUBinding::BindTexture(t));
    self
  }

  pub fn sampler(mut self, s: &'a WGPUSampler) -> Self {
    self.bindings.push(WGPUBinding::BindSampler(s));
    self
  }

  pub fn build(&self, device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> WGPUBindGroup {
    WGPUBindGroup::new(device, &self.bindings, layout)
  }
}

pub struct BindGroupLayoutBuilder {
  pub bindings: Vec<wgpu::BindGroupLayoutEntry>,
}

impl BindGroupLayoutBuilder {
  pub fn new() -> Self {
    Self {
      bindings: Vec::new(),
    }
  }

  pub fn bind(mut self, ty: wgpu::BindingType, visibility: rendiation_ral::ShaderStage) -> Self {
    let binding = self.bindings.len() as u32;
    self.bindings.push(wgpu::BindGroupLayoutEntry {
      binding,
      visibility: shader_stage_convert(visibility),
      ty,
    });
    self
  }

  pub fn build(self, renderer: &WGPURenderer) -> wgpu::BindGroupLayout {
    renderer
      .device
      .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        bindings: &self.bindings,
      })
  }
}
