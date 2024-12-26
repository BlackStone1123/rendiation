use crate::*;

pub fn update_last_frame_visibility_by_all_and_return_objects_that_not_be_occluded_in_this_frame(
  last_frame: &StorageBufferDataView<[Bool]>,
  depth_pyramid: &GPU2DDepthTextureView,
  batch: &DeviceSceneModelRenderBatch,
  pass: &mut GPUComputePass,
  device: &GPUDevice,
) -> DeviceSceneModelRenderBatch {
  let hasher = shader_hasher_from_marker_ty!(OcclusionTestAndUpdate);
  let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |mut ctx| {
    //
    ctx
  });
  todo!()
}

struct OcclusionTester {
  depth_pyramid: HandleNode<ShaderTexture2D>,
  view_projection: Node<Mat4<f32>>,
}

impl OcclusionTester {
  /// return if visible
  fn test_occlusion(&self, target_world_bounding: TargetWorldBounding) -> Node<bool> {
    let size = target_world_bounding.max - target_world_bounding.min;

    let min_xy: Node<Vec2<f32>> = (val(1.), val(1.)).into();
    let max_xy: Node<Vec2<f32>> = (val(0.), val(0.)).into();
    let min_xy = min_xy.make_local_var();
    let max_xy = max_xy.make_local_var();
    let min_z = val(1.).make_local_var();

    val(8).into_shader_iter().for_each(|item, _| {
      let corner_x = target_world_bounding.min.x().make_local_var();
      let corner_y = target_world_bounding.min.y().make_local_var();
      let corner_z = target_world_bounding.min.z().make_local_var();

      switch_by(item)
        .case(1, || {
          corner_x.store(corner_x.load() + size.x());
        })
        .case(2, || {
          corner_y.store(corner_y.load() + size.y());
        })
        .case(3, || {
          corner_z.store(corner_z.load() + size.z());
        })
        .case(4, || {
          corner_x.store(corner_x.load() + size.x());
          corner_y.store(corner_y.load() + size.y());
        })
        .case(5, || {
          corner_y.store(corner_y.load() + size.y());
          corner_z.store(corner_z.load() + size.z());
        })
        .case(6, || {
          corner_x.store(corner_x.load() + size.x());
          corner_z.store(corner_z.load() + size.z());
        })
        .case(7, || {
          corner_x.store(corner_x.load() + size.x());
          corner_y.store(corner_y.load() + size.y());
          corner_z.store(corner_z.load() + size.z());
        })
        .end_with_default(|| {});

      let point: Node<Vec4<f32>> =
        (corner_x.load(), corner_y.load(), corner_z.load(), val(1.)).into();
      let clip_position = self.view_projection * point;

      let pos_xyz = clip_position.xyz() / clip_position.w().splat();
      let x = pos_xyz.x().clamp(val(-1.), val(1.0));
      let y = pos_xyz.y().clamp(val(-1.), val(1.0));
      let z = pos_xyz.z().clamp(val(0.), val(1.0));

      let pos_xy: Node<Vec2<f32>> = (x, y).into();
      let pos_xy = pos_xy * val(Vec2::new(0.5, -0.5)) + val(Vec2::new(0.5, 0.5));

      min_xy.store(min_xy.load().min(pos_xy));
      max_xy.store(max_xy.load().max(pos_xy));
      min_z.store(min_z.load().min(z));
    });

    let min_xy = min_xy.load();
    let max_xy = max_xy.load();

    let depth_pyramid_size_0 = self
      .depth_pyramid
      .texture_dimension_2d(Some(val(0)))
      .into_f32();

    let box_size = (max_xy - min_xy) * depth_pyramid_size_0;
    let mip_level = box_size.x().max(box_size.y()).log2().ceil().into_u32();
    let mip_level = mip_level.clamp(val(0), self.depth_pyramid.texture_number_levels() - val(1));

    let depth_pyramid_size = self.depth_pyramid.texture_dimension_2d(Some(mip_level));
    let limit_x = depth_pyramid_size.x() - val(1);
    let limit_y = depth_pyramid_size.y() - val(1);
    let top_left = (min_xy * depth_pyramid_size.into_f32()).into_u32();

    let l_x = top_left.x().clamp(val(0), limit_x);
    let t_y = top_left.y().clamp(val(0), limit_y);
    let r_x = (l_x + val(1)).clamp(val(0), limit_x);
    let b_y = (t_y + val(1)).clamp(val(0), limit_y);

    let d_0 = self
      .depth_pyramid
      .load_texel((l_x, t_y).into(), mip_level)
      .x();
    let d_1 = self
      .depth_pyramid
      .load_texel((r_x, t_y).into(), mip_level)
      .x();
    let d_2 = self
      .depth_pyramid
      .load_texel((l_x, b_y).into(), mip_level)
      .x();
    let d_3 = self
      .depth_pyramid
      .load_texel((r_x, b_y).into(), mip_level)
      .x();

    let max_depth = d_0.max(d_1).max(d_2).max(d_3);
    min_z.load().less_equal_than(max_depth)
  }
}
