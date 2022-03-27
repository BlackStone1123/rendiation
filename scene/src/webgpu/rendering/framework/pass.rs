use std::marker::PhantomData;

use rendiation_webgpu::{
  ColorChannelView, GPURenderPassCtx, Operations, RenderPassDescriptorOwned,
};

use crate::{Attachment, AttachmentWriteView, FrameCtx, SceneRenderPass};

pub fn pass(name: impl Into<String>) -> PassDescriptor<'static> {
  let mut desc = RenderPassDescriptorOwned::default();
  desc.name = name.into();
  PassDescriptor {
    phantom: PhantomData,
    desc,
  }
}

pub struct PassDescriptor<'a> {
  phantom: PhantomData<&'a Attachment>,
  desc: RenderPassDescriptorOwned,
}

impl<'a> From<AttachmentWriteView<&'a mut Attachment>> for ColorChannelView {
  fn from(val: AttachmentWriteView<&'a mut Attachment>) -> Self {
    val.view
  }
}

impl<'a> PassDescriptor<'a> {
  #[must_use]
  pub fn with_color(
    mut self,
    attachment: impl Into<ColorChannelView> + 'a,
    op: impl Into<wgpu::Operations<wgpu::Color>>,
  ) -> Self {
    self.desc.channels.push((op.into(), attachment.into()));
    self
  }

  #[must_use]
  pub fn with_depth(
    mut self,
    attachment: impl Into<ColorChannelView> + 'a,
    op: impl Into<wgpu::Operations<f32>>,
  ) -> Self {
    self
      .desc
      .depth_stencil_target
      .replace((op.into(), attachment.into()));

    // todo check sample count is same as color's

    self
  }

  #[must_use]
  pub fn resolve_to(mut self, attachment: AttachmentWriteView<&'a mut Attachment>) -> Self {
    self.desc.resolve_target = attachment.view.into();
    self
  }

  #[must_use]
  pub fn render<'x>(self, ctx: &'x mut FrameCtx) -> ActiveRenderPass<'x> {
    let pass = ctx.encoder.begin_render_pass(self.desc.clone());

    let c = GPURenderPassCtx {
      pass,
      gpu: ctx.gpu,
      binding: Default::default(),
    };

    let pass = SceneRenderPass {
      ctx: c,
      resources: ctx.resources,
    };

    ActiveRenderPass {
      desc: self.desc,
      pass,
    }
  }
}

pub trait PassContent {
  fn render(&mut self, pass: &mut SceneRenderPass);
}

impl<T: PassContent> PassContent for Option<T> {
  fn render(&mut self, pass: &mut SceneRenderPass) {
    if let Some(content) = self {
      content.render(pass);
    }
  }
}

pub struct ActiveRenderPass<'p> {
  pass: SceneRenderPass<'p, 'p, 'p>,
  pub desc: RenderPassDescriptorOwned,
}

impl<'p> ActiveRenderPass<'p> {
  #[allow(clippy::return_self_not_must_use)]
  pub fn by(mut self, mut renderable: impl PassContent) -> Self {
    renderable.render(&mut self.pass);
    self
  }
}

pub fn color(r: f64, g: f64, b: f64) -> wgpu::Color {
  wgpu::Color { r, g, b, a: 1. }
}

pub fn all_zero() -> wgpu::Color {
  wgpu::Color {
    r: 0.,
    g: 0.,
    b: 0.,
    a: 0.,
  }
}

pub fn color_same(r: f64) -> wgpu::Color {
  wgpu::Color {
    r,
    g: r,
    b: r,
    a: 1.,
  }
}

pub fn clear<V>(v: V) -> Operations<V> {
  wgpu::Operations {
    load: wgpu::LoadOp::Clear(v),
    store: true,
  }
}

pub fn load<V>() -> Operations<V> {
  wgpu::Operations {
    load: wgpu::LoadOp::Load,
    store: true,
  }
}
