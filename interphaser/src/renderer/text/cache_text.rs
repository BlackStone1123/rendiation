use crate::{renderer::text::WebGPUxTextPrimitive, FontManager};
use std::collections::HashMap;

use super::{
  CacheQueuedResult, GlyphCache, LayoutedTextGlyphs, TextGlyphLayouter, TextHash, TextInfo,
  TextQuadInstance, TextureCacheAction,
};

pub struct TextCache {
  cache: HashMap<TextHash, LayoutedTextGlyphs>,
  queue: HashMap<TextHash, LayoutedTextGlyphs>,
  layouter: Box<dyn TextGlyphLayouter>,
  glyph_cache: GlyphCache,
}

impl TextCache {
  pub fn new(glyph_cache: GlyphCache, layouter: impl TextGlyphLayouter + 'static) -> Self {
    Self {
      cache: Default::default(),
      queue: Default::default(),
      layouter: Box::new(layouter),
      glyph_cache,
    }
  }
}

pub struct TextCacheItem {
  pub layout: LayoutedTextGlyphs,
  pub gpu: WebGPUxTextPrimitive,
}

impl TextCache {
  pub fn queue(&mut self, text: &TextInfo, fonts: &FontManager) -> TextHash {
    let layout = self.layouter.layout(text, fonts);
    let hash = text.hash();
    self.queue.insert(hash, layout);
    hash
  }

  pub fn drop_cache(&mut self, text: TextHash) {
    self.cache.remove(&text);
  }

  pub fn clear_cache(&mut self) {
    self.cache.clear();
  }

  pub fn process_queued(
    &mut self,
    fonts: &FontManager,
    tex_cache_update: impl FnMut(TextureCacheAction) -> bool, // return if cache_resize success
    mut vert_cache_update: impl FnMut(TextHash, Vec<TextQuadInstance>),
  ) {
    self.queue.iter().for_each(|(_, layout)| {
      for (gly_id, ras_info, _) in &layout.glyphs {
        self.glyph_cache.queue_glyph(*gly_id, *ras_info)
      }
    });

    match self
      .glyph_cache
      .process_queued(tex_cache_update, fonts)
      .unwrap()
    {
      CacheQueuedResult::Adding => {
        // build only new queued text
        for (hash, text) in self.queue.drain() {
          vert_cache_update(hash, text.generate_gpu_vertex(&self.glyph_cache));
          self.cache.insert(hash, text);
        }
      }
      CacheQueuedResult::Reordering => {
        // refresh all cached text with new glyph position
        for (hash, text) in self.queue.drain() {
          vert_cache_update(hash, text.generate_gpu_vertex(&self.glyph_cache));
          self.cache.insert(hash, text);
        }
        for (hash, text) in self.queue.drain() {
          vert_cache_update(hash, text.generate_gpu_vertex(&self.glyph_cache));
        }
      }
    }
  }
}
