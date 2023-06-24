use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightShaderInfo {
  /// in lx
  pub illuminance: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub shadow: LightShadowAddressInfo,
}

impl PunctualShaderLight for DirectionalLightShaderInfo {
  type PunctualDependency = ();

  fn create_punctual_dep(
    _: &mut ShaderGraphFragmentBuilderView,
  ) -> Result<Self::PunctualDependency, ShaderGraphBuildError> {
    Ok(())
  }

  fn compute_incident_light(
    builder: &ShaderGraphFragmentBuilderView,
    light: &ENode<Self>,
    _dep: &Self::PunctualDependency,
    _ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> Result<ENode<ShaderIncidentLight>, ShaderGraphBuildError> {
    let shadow_info = light.shadow.expand();
    let occlusion = consts(1.).mutable();

    if_by_ok(shadow_info.enabled.equals(consts(1)), || {
      let map = builder.query::<BasicShadowMap>().unwrap();
      let sampler = builder.query::<BasicShadowMapSampler>().unwrap();

      let shadow_infos = builder.query::<BasicShadowMapInfoGroup>().unwrap();
      let shadow_info = shadow_infos.index(shadow_info.index).expand();

      let shadow_position = compute_shadow_position(builder, shadow_info)?;

      if_by(cull_directional_shadow(shadow_position), || {
        occlusion.set(sample_shadow(
          shadow_position,
          map,
          sampler,
          shadow_info.map_info,
        ))
      });
      Ok(())
    })?;

    Ok(ENode::<ShaderIncidentLight> {
      color: light.illuminance * (consts(1.) - occlusion.get()),
      direction: light.direction,
    })
  }
}

wgsl_fn!(
  /// custom extra culling for directional light
  fn cull_directional_shadow(
    shadow_position: vec3<f32>,
  ) -> bool {
    // maybe we could use sampler's border color config, but that's not part of standard webgpu (wgpu supports)
    let inFrustumVec = vec4<bool>(shadow_position.x >= 0.0, shadow_position.x <= 1.0, shadow_position.y >= 0.0, shadow_position.y <= 1.0);
    let inFrustum = all(inFrustumVec);
    let frustumTestVec = vec2<bool>(inFrustum, shadow_position.z <= 1.0);
    return all(frustumTestVec);
  }
);

impl WebGPULight for SceneItemRef<DirectionalLight> {
  type Uniform = DirectionalLightShaderInfo;

  fn create_uniform_stream(
    &self,
    ctx: &mut LightResourceCtx,
    node: Box<dyn Stream<Item = SceneNode>>,
  ) -> impl Stream<Item = Self::Uniform> {
    enum ShaderInfoDelta {
      Dir(Vec3<f32>),
      Shadow(LightShadowAddressInfo),
      Ill(Vec3<f32>),
    }

    let direction = node
      .map(|node| ctx.derives.create_world_matrix_stream(&node))
      .flatten_signal()
      .map(|mat| mat.forward().reverse().normalize())
      .map(ShaderInfoDelta::Dir);

    let shadow = ctx
      .shadow_system()
      .create_basic_shadow_stream(&self)
      .map(ShaderInfoDelta::Shadow);

    let ill = self
      .single_listen_by(any_change)
      .filter_map_sync(self.defer_weak())
      .map(|light| light.illuminance * light.color_factor)
      .map(ShaderInfoDelta::Ill);

    let delta = futures::stream_select!(direction, shadow, ill);

    delta.fold_signal(DirectionalLightShaderInfo::default(), |delta, info| {
      match delta {
        ShaderInfoDelta::Dir(dir) => info.direction = dir,
        ShaderInfoDelta::Shadow(shadow) => info.shadow = shadow,
        ShaderInfoDelta::Ill(i) => info.illuminance = i,
      };
      Some(())
    })
  }
}

#[derive(Copy, Clone)]
pub struct DirectionalShadowMapExtraInfo {
  pub range: OrthographicProjection<f32>,
  // pub enable_shadow: bool,
}

impl Default for DirectionalShadowMapExtraInfo {
  fn default() -> Self {
    Self {
      range: OrthographicProjection {
        left: -20.,
        right: 20.,
        top: 20.,
        bottom: -20.,
        near: 0.1,
        far: 2000.,
      },
    }
  }
}

impl ShadowSingleProjectCreator for SceneItemRef<DirectionalLight> {
  fn build_shadow_projection(&self) -> Option<impl Stream<Item = Box<dyn CameraProjection>>> {
    let light = self.read();
    let shadow_info = light.ext.get::<DirectionalShadowMapExtraInfo>()?;

    let orth = WorkAroundResizableOrth {
      orth: shadow_info.range,
    };
    let orth = Box::new(orth) as Box<dyn CameraProjection>;

    let proj = CameraProjector::Orthographic(extra.range);
    SceneCamera::create(proj, node.clone())
  }
}
