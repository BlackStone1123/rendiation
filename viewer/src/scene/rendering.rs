use crate::renderer::RenderPassCreator;

use super::*;

pub trait RenderStyle: Sized {
  fn update<'a>(
    m: &mut dyn Material,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<'a, Self>,
  );
  fn setup_pass<'a>(
    m: &'a dyn Material,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a, Self>,
  );
}

pub struct OriginForward;
impl RenderStyle for OriginForward {
  fn update<'a>(
    m: &mut dyn Material,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<'a, Self>,
  ) {
    m.update(renderer, ctx)
  }

  fn setup_pass<'a>(
    m: &'a dyn Material,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a, Self>,
  ) {
    m.setup_pass(pass, ctx)
  }
}

impl<'b, S> RenderPassCreator<wgpu::SwapChainFrame> for RenderPassDispatcher<'b, S> {
  fn create<'a>(
    &self,
    target: &'a wgpu::SwapChainFrame,
    encoder: &'a mut wgpu::CommandEncoder,
  ) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: "scene pass".into(),
      color_attachments: &[wgpu::RenderPassColorAttachment {
        view: &target.output.view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: self.scene.get_main_pass_load_op(),
          store: true,
        },
      }],
      depth_stencil_attachment: None,
    })
  }
}

impl Scene {
  fn get_main_pass_load_op(&self) -> wgpu::LoadOp<wgpu::Color> {
    if let Some(clear_color) = self.background.require_pass_clear() {
      return wgpu::LoadOp::Clear(clear_color);
    }

    return wgpu::LoadOp::Load;
  }
}

pub struct RenderList {
  models: Vec<ModelHandle>,
}

impl RenderList {
  pub fn new() -> Self {
    Self { models: Vec::new() }
  }
}

pub struct RenderPassDispatcher<'a, S> {
  pub scene: &'a mut Scene,
  pub style: &'a mut S,
}

impl<'a, S: RenderStyle> Renderable for RenderPassDispatcher<'a, S> {
  fn setup_pass<'p>(&'p mut self, pass: &mut wgpu::RenderPass<'p>) {
    let scene = &self.scene;
    let models = &scene.models;
    let ctx = ModelPassSetupContext {
      materials: &scene.materials,
      meshes: &scene.meshes,
      material_ctx: SceneMaterialPassSetupCtx {
        style: self.style,
        camera_gpu: scene.active_camera_gpu.as_ref().unwrap(),
        pipelines: &scene.pipeline_resource,
      },
    };
    scene.render_list.models.iter().for_each(|model| {
      let model = models.get(*model).unwrap();
      model.setup_pass(pass, &ctx)
    })
  }

  fn update(&mut self, renderer: &Renderer, encoder: &mut wgpu::CommandEncoder) {
    let scene = &mut self.scene;
    scene.render_list.models.clear();
    let root = scene.get_root_handle();

    if let Some(active_camera) = &scene.active_camera {
      let camera_gpu = scene
        .active_camera_gpu
        .get_or_insert_with(|| CameraBindgroup::new(renderer, active_camera));

      scene
        .nodes
        .traverse_mut(root, &mut Vec::new(), |this, parent| {
          let node_data = this.data_mut();
          node_data.hierarchy_update(parent.map(|p| p.data()));

          node_data.payloads.iter().for_each(|payload| match payload {
            SceneNodePayload::Model(model) => {
              scene.render_list.models.push(*model);

              let model = scene.models.get_mut(*model).unwrap();
              let material = scene.materials.get_mut(model.material).unwrap().as_mut();
              let mesh = scene.meshes.get_mut(model.mesh).unwrap();

              let mut ctx = SceneMaterialRenderPrepareCtx {
                active_camera,
                camera_gpu,
                model_matrix: &node_data.world_matrix,
                pipelines: &mut scene.pipeline_resource,
                style: self.style,
                active_mesh: mesh,
                textures: &mut scene.texture_2ds,
                samplers: &mut scene.samplers,
              };
              S::update(material, renderer, &mut ctx);
            }
            _ => {}
          });

          if node_data.net_visible {
            NextTraverseVisit::SkipChildren
          } else {
            NextTraverseVisit::VisitChildren
          }
        });
    }
  }
}
