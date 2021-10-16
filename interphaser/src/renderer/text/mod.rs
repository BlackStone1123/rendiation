mod pipeline;
use pipeline::*;
mod text_quad_instance;
use rendiation_algebra::Vec2;
use rendiation_texture::Size;
use text_quad_instance::*;

use glyph_brush::{
  ab_glyph::{self},
  BrushAction, BrushError, DefaultSectionHasher, Extra, GlyphBrushBuilder, GlyphCruncher, Section,
};

use crate::FontManager;

use super::text_next::GPUGlyphCache;

pub struct GPUxUITextPrimitive {
  vertex_buffer: wgpu::Buffer,
  length: u32,
}

pub struct TextRenderer {
  pipeline: TextRendererPipeline,
  glyph_brush: glyph_brush::GlyphBrush<Instance, Extra, ab_glyph::FontArc, DefaultSectionHasher>,
  _exp: GPUGlyphCache,
}

impl TextRenderer {
  pub fn new(
    device: &wgpu::Device,
    filter_mode: wgpu::FilterMode,
    render_format: wgpu::TextureFormat,
    fonts: &FontManager,
  ) -> Self {
    let glyph_brush = GlyphBrushBuilder::using_fonts(fonts.get_fonts().clone())
      .cache_redraws(false)
      .build();

    let size = Size::from_u32_pair_min_one(glyph_brush.texture_dimensions());
    Self {
      pipeline: TextRendererPipeline::new(
        device,
        filter_mode,
        render_format,
        size,
        Vec2::new(1000., 1000.),
      ),
      _exp: GPUGlyphCache::new(device),
      glyph_brush,
    }
  }

  pub fn update_fonts(&mut self, fonts: &FontManager) {
    if fonts.active_font_count() != self.glyph_brush.fonts().len() {
      self.glyph_brush = GlyphBrushBuilder::using_fonts(fonts.get_fonts().clone())
        .cache_redraws(false)
        .build()
    }
  }

  pub fn resize_view(&mut self, size: Vec2<f32>, queue: &wgpu::Queue) {
    self.pipeline.resize_view(size, queue)
  }

  pub fn draw_gpu_text<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    text: &'a GPUxUITextPrimitive,
  ) {
    self.pipeline.draw(pass, text)
  }

  pub fn create_gpu_text<'a>(
    &mut self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    section: Section<'a, Extra>,
  ) -> Option<GPUxUITextPrimitive> {
    self.glyph_brush.queue(section);
    self.process_queued(device, encoder)
  }

  fn process_queued(
    &mut self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
  ) -> Option<GPUxUITextPrimitive> {
    let brush_action = self.glyph_brush.process_queued(
      |rect, tex_data| {
        let offset = (rect.min[0], rect.min[1]);

        let tex_data = TextureWriteData {
          data: tex_data,
          size: Size::from_u32_pair_min_one((rect.width(), rect.height())),
        };

        self
          .pipeline
          .update_cache(device, encoder, offset, tex_data);
      },
      Instance::from_vertex,
    );

    match brush_action {
      Ok(brush_action) => match brush_action {
        BrushAction::Draw(verts) => {
          return self.pipeline.create_gpu_text(device, &verts);
        }
        BrushAction::ReDraw => {}
      },
      Err(BrushError::TextureTooSmall { suggested }) => {
        // TODO: Obtain max texture dimensions using `wgpu`
        // This is currently not possible I think. Ask!
        let max_image_dimension = 2048;

        let (new_width, new_height) = if (suggested.0 > max_image_dimension
          || suggested.1 > max_image_dimension)
          && (self.glyph_brush.texture_dimensions().0 < max_image_dimension
            || self.glyph_brush.texture_dimensions().1 < max_image_dimension)
        {
          (max_image_dimension, max_image_dimension)
        } else {
          suggested
        };

        log::warn!(
          "Increasing glyph texture size {old:?} -> {new:?}. \
                             Consider building with `.initial_cache_size({new:?})` to avoid \
                             resizing",
          old = self.glyph_brush.texture_dimensions(),
          new = (new_width, new_height),
        );

        let size = Size::from_u32_pair_min_one((new_width, new_height));
        self.pipeline.increase_cache_size(device, size);
        self.glyph_brush.resize_texture(new_width, new_height);
      }
    }
    None
  }
}
