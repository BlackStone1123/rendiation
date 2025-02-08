use crate::*;

pub const LINE_DRAW_CMD: DrawCommand = DrawCommand::Array {
  vertices: 0..2,
  instances: 0..1,
};

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderLine {
  pub point: Vec3<f32>,
  pub direction: Vec3<f32>,
}

impl ShaderLine {
  pub fn new(point: Vec3<f32>, direction: Vec3<f32>) -> Self {
    Self {
      point,
      direction,
      ..Zeroable::zeroed()
    }
  }
}

pub struct InfinityShaderLineEffect<'a> {
  pub line: &'a UniformBufferCachedDataView<ShaderLine>,
  pub camera: &'a dyn RenderComponent,
}

impl ShaderHashProvider for InfinityShaderLineEffect<'_> {
  shader_hash_type_id! {InfinityShaderPlaneEffect<'static>}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.camera.hash_pipeline(hasher);
  }
}
impl ShaderPassBuilder for InfinityShaderLineEffect<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.camera.setup_pass(ctx);
    ctx.binding.bind(self.line);
  }
}

impl GraphicsShaderProvider for InfinityShaderLineEffect<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.camera.build(builder);

    builder.vertex(|builder, bind| {
      let vertex_index = builder.query::<VertexIndex>();
      let view_proj = builder.query::<CameraViewProjectionMatrix>();
      let normal_mat = builder
        .query::<CameraWorldMatrix>()
        .shrink_to_3()
        .transpose();

      let line = bind.bind_by(&self.line).load().expand();
      let point: Node<Vec4<f32>> = (line.point, val(1.)).into();
      let origin_in_ndc = view_proj * point;
      let origin_in_ndc = origin_in_ndc.xyz() / origin_in_ndc.w().splat();
      let direct_in_ndc = (normal_mat * line.direction).normalize();

      let position = val(Vec3::zero()).make_local_var();
      switch_by(vertex_index)
        .case(0, || {
          position.store(origin_in_ndc);
        })
        .end_with_default(|| {
          let far_plane = ENode::<ShaderPlane> {
            normal: val(Vec3::new(0., 0., 1.)),
            constant: val(1.),
          };

          let near_plane = ENode::<ShaderPlane> {
            normal: val(Vec3::new(0., 0., 1.)),
            constant: val(0.),
          };

          let far = ray_plane_intersect(origin_in_ndc, direct_in_ndc, far_plane);
          let far_hit = far.w().equals(0.).select(origin_in_ndc, far.xyz());
          let near = ray_plane_intersect(origin_in_ndc, direct_in_ndc, near_plane);
          let near_hit = near.w().equals(0.).select(origin_in_ndc, near.xyz());

          let far_distance = (far_hit - origin_in_ndc).length();
          let near_distance = (near_hit - origin_in_ndc).length();

          let hit_choice = far_distance
            .less_than(near_distance)
            .select(near_hit, far_hit);

          position.store(hit_choice);
        });

      builder.register::<ClipPosition>((position.load(), val(1.)));

      builder.primitive_state = PrimitiveState {
        topology: PrimitiveTopology::LineList,
        ..Default::default()
      };
    });
  }
}
