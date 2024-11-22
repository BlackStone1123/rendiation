use crate::*;

pub struct RayTracingAORenderSystem {
  camera: DefaultRtxCameraRenderImplProvider,
  sbt: UpdateResultToken,
  executor: GPURaytracingPipelineExecutor,
  scene_tlas: UpdateResultToken, // todo, share, unify the share mechanism with the texture
  system: RtxSystemCore,
  ao_buffer: Option<GPU2DTextureView>,
}

impl RayTracingAORenderSystem {
  pub fn new(rtx: &RtxSystemCore) -> Self {
    Self {
      camera: Default::default(),
      scene_tlas: Default::default(),
      sbt: Default::default(),
      executor: rtx.rtx_device.create_raytracing_pipeline_executor(),
      system: rtx.clone(),
      ao_buffer: None,
    }
  }
}

impl RenderImplProvider<SceneRayTracingAORenderer> for RayTracingAORenderSystem {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.scene_tlas = source.register_reactive_query(scene_to_tlas(self.system.rtx_acc.clone()));

    // todo support max mesh count grow
    let sbt = GPUSbt::new(self.system.rtx_device.create_sbt(2000, 2));
    let sbt = MultiUpdateContainer::new(sbt);
    // todo, add sbt maintain logic here
    // .with_source(source);

    self.sbt = source.register_multi_updater(sbt);
    self.camera.register_resource(source, cx);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.scene_tlas);
    source.deregister(&mut self.sbt);
    self.camera.deregister_resource(source);
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> SceneRayTracingAORenderer {
    let sbt = res.take_multi_updater_updated::<GPUSbt>(self.sbt).unwrap();
    SceneRayTracingAORenderer {
      executor: self.executor.clone(),
      sbt: sbt.target.clone(),
      scene_tlas: res.take_reactive_query_updated(self.scene_tlas).unwrap(),
      camera: self.camera.create_impl(res),
      rtx_system: self.system.rtx_system.clone(),
      ao_buffer: self.ao_buffer.clone(),
    }
  }
}

pub struct SceneRayTracingAORenderer {
  camera: Box<dyn RtxCameraRenderImpl>,
  executor: GPURaytracingPipelineExecutor,
  sbt: GPUSbt,
  rtx_system: Box<dyn GPURaytracingSystem>,
  scene_tlas: BoxedDynQuery<EntityHandle<SceneEntity>, TlasHandle>,
  ao_buffer: Option<GPU2DTextureView>,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
enum AOTestRayType {
  Primary = 0,
  AOTest = 1,
}

impl AOTestRayType {
  fn to_sbt_cfg(self) -> RaySBTConfig {
    RaySBTConfig {
      offset: val(self as u32),
      stride: val(2),
    }
  }
}

impl SceneRayTracingAORenderer {
  pub fn render(
    &mut self,
    frame: &mut FrameCtx,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> GPU2DTextureView {
    let scene_tlas = self.scene_tlas.access(&scene).unwrap();

    if let Some(ao_buffer) = &self.ao_buffer {
      if ao_buffer.size() != frame.frame_size() {
        self.ao_buffer = None;
      }
    }
    let ao_buffer = self.ao_buffer.clone().unwrap_or_else(|| todo!());
    let ao_buffer_rw = ao_buffer
      .clone()
      .into_storage_texture_view_readwrite()
      .unwrap();

    let mut desc = GPURaytracingPipelineAndBindingSource::default();

    let camera = self.camera.get_rtx_camera(camera);

    let trace_base_builder = self.rtx_system.create_tracer_base_builder();

    let ray_gen_shader = trace_base_builder
      .create_ray_gen_shader_base()
      .inject_ctx(RayTracingAORayGenCtx {
        camera,
        ao_buffer: ao_buffer_rw,
        scene: scene_tlas,
      })
      .then_trace(|_, ctx| {
        let rg_cx = ctx.expect_ray_gen_ctx();
        let cx = ctx.expect_custom_cx::<RayTracingAORayGenCtxInvocation>();
        let normalized_position =
          rg_cx.launch_id().into_f32().xy() / rg_cx.launch_size().into_f32().xy();
        let ray = cx.camera.generate_ray(normalized_position);

        let trace_call = ShaderRayTraceCall {
          tlas_idx: val(todo!()),
          ray_flags: val(RayFlagConfigRaw::RAY_FLAG_CULL_BACK_FACING_TRIANGLES as u32),
          cull_mask: val(u32::MAX),
          sbt_ray_config: AOTestRayType::Primary.to_sbt_cfg(),
          miss_index: val(0),
          ray,
          range: ShaderRayRange::default(),
        };

        (val(true), trace_call, val(0.))
      })
      .map(|(_, payload), ctx| {
        let cx = ctx.expect_custom_cx::<RayTracingAORayGenCtxInvocation>();
        let position = ctx.expect_ray_gen_ctx().launch_id().xy();

        cx.ao_buffer
          .write_texel(position, (payload, payload, payload, val(1.0)).into());
      });

    type RayGenTracePayload = f32; // occlusion
    let ao_closest = RayTracingAOComputeTraceOperator {
      base: trace_base_builder.create_closest_hit_shader_base::<RayGenTracePayload>(),
      scene: scene_tlas,
      max_sample_count: 8,
    };

    desc.register_ray_gen::<u32>(ShaderFutureProviderIntoTraceOperator(ray_gen_shader));
    desc.register_ray_closest_hit::<u32>(ShaderFutureProviderIntoTraceOperator(ao_closest));
    // desc.register_ray_any_hit(builder);
    // desc.register_ray_miss(ray_logic)

    let mut rtx_encoder = self.rtx_system.create_raytracing_encoder();

    let canvas_size = frame.frame_size().into_u32();
    let sbt = self.sbt.inner.read();
    rtx_encoder.trace_ray(
      &desc,
      &self.executor,
      (canvas_size.0, canvas_size.1, 1),
      (*sbt).as_ref(),
    );

    ao_buffer
  }
}

#[derive(Clone)]
struct RayTracingAORayGenCtx {
  camera: Box<dyn RtxCameraRenderComponent>,
  ao_buffer: StorageTextureReadWrite<GPU2DTextureView>,
  scene: TlasHandle,
}

impl ShaderHashProvider for RayTracingAORayGenCtx {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.camera.hash_pipeline(hasher);
  }
}

impl RayTracingCustomCtxProvider for RayTracingAORayGenCtx {
  type Invocation = RayTracingAORayGenCtxInvocation;

  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation {
    RayTracingAORayGenCtxInvocation {
      camera: self.camera.build_invocation(cx),
      ao_buffer: cx.bind_by(&self.ao_buffer),
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.camera.bind(builder);
  }
}

#[derive(Clone)]
struct RayTracingAORayGenCtxInvocation {
  camera: Box<dyn RtxCameraRenderInvocation>,
  ao_buffer: HandleNode<ShaderStorageTextureRW2D>,
}

#[derive(Clone)]
struct RayTracingAOComputeTraceOperator {
  base: Box<dyn TraceOperator<()>>,
  max_sample_count: u32,
  scene: TlasHandle,
}

impl ShaderHashProvider for RayTracingAOComputeTraceOperator {
  shader_hash_type_id! {}
}

impl ShaderFutureProvider for RayTracingAOComputeTraceOperator {
  type Output = ();
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<Self::Output> {
    RayTracingAOComputeFuture {
      upstream: self.base.build_device_future(ctx),
      max_sample_count: self.max_sample_count,
      tracing: TracingFuture::default(),
    }
    .into_dyn()
  }
}

struct RayTracingAOComputeFuture {
  upstream: DynShaderFuture<()>,
  max_sample_count: u32,
  tracing: TracingFuture<f32>,
}

impl ShaderFuture for RayTracingAOComputeFuture {
  type Output = ();

  type Invocation = RayTracingAOComputeInvocation;

  fn required_poll_count(&self) -> usize {
    self.upstream.required_poll_count() + 1
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    RayTracingAOComputeInvocation {
      upstream: self.upstream.build_poll(ctx),
      max_sample_count: self.max_sample_count,
      hit_position: ctx.make_state::<Node<Vec3<f32>>>(),
      hit_normal: ctx.make_state::<Node<Vec3<f32>>>(),
      next_sample_idx: ctx.make_state::<Node<u32>>(),
      occlusion_acc: ctx.make_state::<Node<f32>>(),
      trace_on_the_fly: self.tracing.build_poll(ctx),
    }
  }

  fn bind_input(&self, _builder: &mut DeviceTaskSystemBindCtx) {
    todo!()
  }
}

struct RayTracingAOComputeInvocation {
  upstream: Box<dyn ShaderFutureInvocation<Output = ()>>,
  max_sample_count: u32,
  hit_position: BoxedShaderLoadStore<Node<Vec3<f32>>>,
  hit_normal: BoxedShaderLoadStore<Node<Vec3<f32>>>,
  next_sample_idx: BoxedShaderLoadStore<Node<u32>>,
  occlusion_acc: BoxedShaderLoadStore<Node<f32>>,
  trace_on_the_fly: TracingFutureInvocation<f32>,
}

impl ShaderFutureInvocation for RayTracingAOComputeInvocation {
  type Output = ();

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    let _ = self.upstream.device_poll(ctx); // upstream has no real runtime logic, let's skip the check

    let current_idx = self.next_sample_idx.abstract_load();
    let sample_is_done = current_idx.greater_equal_than(self.max_sample_count);

    if_by(current_idx.equals(0), || {
      let closest_hit_ctx = ctx
        .invocation_registry
        .get::<TracingCtx>()
        .unwrap()
        .closest_hit_ctx()
        .unwrap();

      let hit_position = closest_hit_ctx.world_ray().origin
        + closest_hit_ctx.world_ray().direction * closest_hit_ctx.hit_distance();
      let hit_normal = todo!();

      self.hit_position.abstract_store(hit_position);
      self.hit_normal.abstract_store(hit_normal);
    });

    // self.camera.generate_ray(normalized_position);

    if_by(sample_is_done.not(), || {
      let ray = todo!();

      let trace_call = ShaderRayTraceCall {
        tlas_idx: val(todo!()),
        ray_flags: val(RayFlagConfigRaw::RAY_FLAG_CULL_BACK_FACING_TRIANGLES as u32),
        cull_mask: val(u32::MAX),
        sbt_ray_config: AOTestRayType::AOTest.to_sbt_cfg(),
        miss_index: val(0),
        ray,
        range: ShaderRayRange::default(),
      };

      // let r = self.next_trace_on_the_fly.try_spawn_and_poll(ctx);
      // if_by(r.is_ready, || {
      //   self.next_sample_idx.abstract_store(current_idx + val(1));
      // });
      //
    });

    let occlusion = self.occlusion_acc.abstract_load() / val(self.max_sample_count as f32);
    (sample_is_done, ()).into()
  }
}
