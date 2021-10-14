use rendiation_webgpu::GPU;

use crate::*;

pub struct BackGroundRendering;

impl PassContent for BackGroundRendering {
  fn update(
    &mut self,
    gpu: &GPU,
    scene: &mut Scene,
    _resource: &mut ResourcePoolInner,
    pass_info: &PassTargetFormatInfo,
  ) {
    if let Some(active_camera) = &mut scene.active_camera {
      let (active_camera, camera_gpu) = active_camera.get_updated_gpu(gpu, &scene.nodes);

      let mut base = SceneMaterialRenderPrepareCtxBase {
        active_camera,
        camera_gpu,
        pass: pass_info,
        pipelines: &mut scene.pipeline_resource,
        layouts: &mut scene.layouts,
        textures: &mut scene.texture_2ds,
        texture_cubes: &mut scene.texture_cubes,
        samplers: &mut scene.samplers,
        reference_finalization: &scene.reference_finalization,
      };

      scene.background.update(
        gpu,
        &mut base,
        &mut scene.materials,
        &mut scene.meshes,
        &mut scene.nodes,
      );
    }
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    scene: &'a Scene,
    pass_info: &'a PassTargetFormatInfo,
  ) {
    scene.background.setup_pass(
      pass,
      &scene.materials,
      &scene.meshes,
      &scene.nodes,
      scene.active_camera.as_ref().unwrap().expect_gpu(),
      &scene.pipeline_resource,
      pass_info,
    );
  }
  //
}
