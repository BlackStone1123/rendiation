use std::collections::HashMap;

use crate::rinecraft::RinecraftState;
use rendiation_ral::ResourceManager;
use rendiation_rendergraph::{RenderGraph, RenderGraphExecutor, WebGPURenderGraphBackend};
use rendiation_scenegraph::{default_impl::DefaultSceneBackend, DrawcallList, Scene};
use rendiation_webgpu::{
  renderer::SwapChain, RenderTargetAble, ScreenRenderTarget, ScreenRenderTargetInstance,
  WGPURenderPassBuilder, WGPURenderer,
};
use rendium::EventCtx;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectConfig {
  enable_grain: bool,
}

pub struct RinecraftRenderer {
  cache: HashMap<EffectConfig, RenderGraph<WebGPURenderGraphBackend>>,
  executor: RenderGraphExecutor<WebGPURenderGraphBackend>,
}

impl RinecraftRenderer {
  pub fn new() -> Self {
    Self {
      cache: HashMap::new(),
      executor: RenderGraphExecutor::new(),
    }
  }

  fn render_experiment(
    &mut self,
    renderer: &mut WGPURenderer,
    target: &ScreenRenderTargetInstance,
    config: &EffectConfig,
  ) {
    let graph = self
      .cache
      .entry(*config)
      .or_insert_with(|| Self::build(config));
    let target = unsafe { std::mem::transmute(&target) };
    self.executor.render(graph, target, renderer);
  }

  fn build(config: &EffectConfig) -> RenderGraph<WebGPURenderGraphBackend> {
    let graph = RenderGraph::new();

    // let normal_pass = graph.pass("normal");
    // let normal_target = graph.target("normal").from_pass(&normal_pass);
    // let copy_screen = graph
    //   .pass("copy_screen")
    //   .depend(&normal_target)
    //   .render_by(|_, _| {
    //     let _a = 1;
    //   });

    let scene_pass = graph
      .pass("scene-pass")
      .define_pass_ops(|b: WGPURenderPassBuilder| {
        b.first_color(|c| c.load_with_clear((0.1, 0.2, 0.3).into(), 1.0).ok())
          .depth(|d| d.load_with_clear(1.0).ok())
      })
      .render_by(|_, pass| {
        todo!();
        todo!()
      });

    graph.finally().from_pass(&scene_pass);
    graph
  }

  pub fn render(
    &mut self,
    renderer: &mut WGPURenderer,
    scene: &mut Scene<WGPURenderer>,
    resource: &mut ResourceManager<WGPURenderer>,
    output: &ScreenRenderTargetInstance,
  ) {
    let list = scene.update(resource);
    resource.maintain_gpu(renderer);

    {
      let mut pass = output
        .create_render_pass_builder()
        .first_color(|c| c.load_with_clear((0.1, 0.2, 0.3).into(), 1.0).ok())
        .depth(|d| d.load_with_clear(1.0).ok())
        .create(&mut renderer.encoder);

      list.render(unsafe { std::mem::transmute(&mut pass) }, scene, resource);
    }

    renderer
      .queue
      .submit(&renderer.device, &mut renderer.encoder);
  }
}
