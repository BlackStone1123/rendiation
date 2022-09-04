use crate::*;

pub fn get_main_pass_load_op<S>(scene: &Scene<S>) -> webgpu::Operations<webgpu::Color>
where
  S: SceneContent,
  S::BackGround: Deref<Target = dyn WebGPUBackground>,
{
  let load = if let Some(clear_color) = scene.background.as_ref().unwrap().require_pass_clear() {
    webgpu::LoadOp::Clear(clear_color)
  } else {
    webgpu::LoadOp::Load
  };

  webgpu::Operations { load, store: true }
}

pub struct ForwardScene;

impl<S> PassContentWithSceneAndCamera<S> for ForwardScene
where
  S: SceneContent,
  S::Model: Deref<Target = dyn SceneModelShareable>,
{
  fn render(&mut self, pass: &mut SceneRenderPass, scene: &Scene<S>, camera: &SceneCamera) {
    let mut render_list = RenderList::<S>::default();
    render_list.prepare(scene, camera);
    render_list.setup_pass(pass, scene, &pass.default_dispatcher(), camera);
  }
}

/// contains gpu data that support forward rendering
pub struct ForwardLightingSystem {
  pub lights: Vec<Box<dyn Any>>,
}

impl ForwardLightingSystem {
  pub fn update_by_scene(&mut self, scene: &Scene<WebGPUScene>) {
    for (_, light) in &scene.lights {
      let light = &light.read().light;

      //
    }
  }
}

pub struct LightList<T: ShaderLight> {
  pub lights: Vec<T>,
  pub lights_gpu: UniformBufferDataView<Shader140Array<T, 32>>,
}

impl<T: ShaderLight> LightList<T> {
  pub fn update(&mut self) {
    //
  }

  pub fn collect_lights_for_naive_forward<S: LightableSurfaceShading>(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
    shading: &ExpandedNode<S>,
    geom_ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let lights = binding.uniform_by(&self.lights_gpu, SB::Pass);

      // let camera_position = builder.query::<FragmentWorldPosition>()?.get(); // todo
      // let geom_position = builder.query::<FragmentWorldPosition>()?.get();

      // let ctx = ExpandedNode::<ShaderLightingGeometricCtx> {
      //   position: geom_position,
      //   normal: builder.query::<FragmentWorldNormal>()?.get(),
      //   view_dir: camera_position - geom_position,
      // };
      let dep = T::create_dep(builder);

      // let shading = S::construct_shading(builder);

      let light_specular_result = consts(Vec3::zero()).mutable();
      let light_diffuse_result = consts(Vec3::zero()).mutable();

      for_by(lights, |_, light| {
        let light = light.expand();
        let incident = T::compute_direct_light(&light, &dep, geom_ctx);
        let light_result = S::compute_lighting(shading, &incident, geom_ctx);

        // improve impl add assign
        light_specular_result.set(light_specular_result.get() + light_result.specular);
        light_diffuse_result.set(light_diffuse_result.get() + light_result.diffuse);
      });

      let hdr_result = ExpandedNode::<ShaderLightingResult> {
        diffuse: light_diffuse_result.get(),
        specular: light_specular_result.get(),
      }
      .construct();

      builder.register::<HDRLightResult>(hdr_result);

      Ok(())
    })
  }
}
