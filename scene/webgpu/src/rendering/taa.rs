// https://www.elopezr.com/temporal-aa-and-the-quest-for-the-holy-trail/#more-3285

use crate::*;

pub struct TAA {
  history: Attachment,
  current_camera: CameraGPU,
  previous_camera: CameraGPU,
}

impl TAA {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      history: todo!(),
      current_camera: todo!(),
      previous_camera: todo!(),
    }
  }
  pub fn resolve(
    &mut self,
    new_color: &Attachment,
    new_depth: &Attachment,
    ctx: &mut FrameCtx,
    camera: &SceneCamera,
  ) -> &Attachment {
    // refresh cameras:
    let new_camera = ctx.resources.cameras.check_update_gpu(camera, ctx.gpu);

    // improve? i think we could try copy buffer to buffer here.
    self
      .previous_camera
      .ubo
      .resource
      .copy_cpu(&self.current_camera.ubo.resource)
      .upload(&ctx.gpu.queue);

    self
      .current_camera
      .ubo
      .resource
      .copy_cpu(&new_camera.ubo.resource)
      .upload(&ctx.gpu.queue);

    let mut resolve_target = attachment()
      .format(webgpu::TextureFormat::Rgba8Unorm)
      .request(ctx);

    pass("taa-resolve")
      .with_color(resolve_target.write(), load())
      .render(ctx)
      .by(
        TAAResolver {
          history: self.history.read(),
          new_color: new_color.read(),
          new_depth: new_depth.read(),
          current_camera: &self.current_camera,
          previous_camera: &self.previous_camera,
        }
        .draw_quad(),
      );

    // note, if the history size different than current, it's still works fine
    // and the history will be correct update to new size
    std::mem::swap(&mut self.history, &mut resolve_target);

    &self.history
  }
}

struct TAAResolver<'a> {
  history: AttachmentView<&'a Attachment>,
  new_color: AttachmentView<&'a Attachment>,
  new_depth: AttachmentView<&'a Attachment>,
  current_camera: &'a CameraGPU,
  previous_camera: &'a CameraGPU,
}

impl<'a> ShaderGraphProvider for TAAResolver<'a> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let sampler = binding.uniform::<GPUSamplerView>(SB::Material);
      let history = binding.uniform_by(&self.history, SB::Material);
      let new = binding.uniform_by(&self.new_color, SB::Material);
      let new_depth = binding.uniform_by(&self.new_depth, SB::Material);

      let current_camera = binding
        .uniform_by(&self.current_camera.ubo, SB::Material)
        .expand();

      let previous_camera = binding
        .uniform_by(&self.previous_camera.ubo, SB::Material)
        .expand();

      let uv = builder.query::<FragmentUv>()?;

      let depth = new_depth.sample(sampler, uv).x();
      let xy = uv * consts(2.) - consts(Vec2::one());
      let position_in_current_ndc = (xy, depth, 1.).into();

      let world_position = current_camera.view_projection_inv * position_in_current_ndc;
      let position_in_previous_ndc = previous_camera.view_projection * world_position;
      let position_in_previous_ndc = position_in_previous_ndc.xyz() / position_in_previous_ndc.w();

      let reproject_uv = position_in_previous_ndc.xy() * consts(0.5) + consts(Vec2::splat(0.5));
      let previous = history.sample(sampler, reproject_uv);

      let new = new.sample(sampler, uv);

      let output = new * consts(0.1) + previous * consts(0.9);

      builder.set_fragment_out(0, output)
    })
  }
}

impl<'a> ShaderPassBuilder for TAAResolver<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.bind_immediate_sampler(&TextureSampler::default(), SB::Material);
    ctx.binding.bind(&self.history, SB::Material);
    ctx.binding.bind(&self.new_color, SB::Material);
    ctx.binding.bind(&self.new_depth, SB::Material);
    ctx.binding.bind(&self.current_camera.ubo, SB::Material);
    ctx.binding.bind(&self.previous_camera.ubo, SB::Material);
  }
}
impl<'a> ShaderHashProvider for TAAResolver<'a> {}
impl<'a> ShaderHashProviderAny for TAAResolver<'a> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    struct Marker;
    Marker.type_id().hash(hasher)
  }
}
