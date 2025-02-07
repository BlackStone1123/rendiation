use fast_hash_collection::FastHashMap;
use rendiation_mesh_core::{AttributeSemantic, AttributesMeshData};

use crate::*;

/// query all camera in scene and maintain the helper models in scene
pub struct SceneCameraHelper {
  helper_models: FastHashMap<EntityHandle<SceneCameraEntity>, UIWidgetModel>,
  camera_changes: BoxedDynReactiveQuery<EntityHandle<SceneCameraEntity>, Mat4<f32>>,
}

impl SceneCameraHelper {
  pub fn new(
    _scene: EntityHandle<SceneEntity>, // todo, filter other scene
    camera: impl ReactiveQuery<Key = EntityHandle<SceneCameraEntity>, Value = CameraTransform>,
  ) -> Self {
    Self {
      helper_models: Default::default(),
      camera_changes: camera.collective_map(|t| t.view_projection).into_boxed(),
    }
  }
}

impl Widget for SceneCameraHelper {
  fn update_state(&mut self, _cx: &mut DynCx) {}

  fn update_view(&mut self, cx: &mut DynCx) {
    access_cx_mut!(cx, scene_cx, SceneWriter);

    let waker = futures::task::noop_waker_ref();
    let mut cx = Context::from_waker(waker);
    let cx = &mut cx;

    let (changes, _) = self.camera_changes.poll_changes(cx);

    for (k, c) in changes.iter_key_value() {
      match c {
        ValueChange::Remove(_) => {
          let mut model = self.helper_models.remove(&k).unwrap();
          model.do_cleanup(scene_cx);
        }
        ValueChange::Delta(new, _) => {
          let new_mesh = build_debug_line_in_camera_space(new);
          if let Some(helper) = self.helper_models.get_mut(&k) {
            helper.replace_new_shape_and_cleanup_old(scene_cx, new_mesh);
          } else {
            self
              .helper_models
              .insert(k, UIWidgetModel::new(scene_cx, new_mesh));
          }
        }
      }
    }
  }

  fn clean_up(&mut self, cx: &mut DynCx) {
    self.helper_models.values_mut().for_each(|m| m.clean_up(cx));
  }
}

fn build_debug_line_in_camera_space(project_mat: Mat4<f32>) -> AttributesMeshData {
  let zero = 0.0001;
  let one = 0.9999;

  let near = zero;
  let far = one;
  let left = -one;
  let right = one;
  let top = one;
  let bottom = -one;

  let min = Vec3::new(near, left, bottom);
  let max = Vec3::new(far, right, top);

  let lines: Vec<_> = line_box(min, max)
    .into_iter()
    .map(|[a, b]| [project_mat * a, project_mat * b])
    .collect();
  let lines = cast_vec(lines);

  AttributesMeshData {
    attributes: vec![(AttributeSemantic::Positions, lines)],
    indices: None,
    mode: rendiation_mesh_core::PrimitiveTopology::LineList,
    groups: Default::default(),
  }
}

fn line_box(min: Vec3<f32>, max: Vec3<f32>) -> impl IntoIterator<Item = [Vec3<f32>; 2]> {
  let near = min.x;
  let far = max.x;
  let left = min.z;
  let right = max.z;
  let top = max.y;
  let bottom = min.y;

  let near_left_down = Vec3::new(left, bottom, near);
  let near_left_top = Vec3::new(left, top, near);
  let near_right_down = Vec3::new(right, bottom, near);
  let near_right_top = Vec3::new(right, top, near);

  let far_left_down = Vec3::new(left, bottom, far);
  let far_left_top = Vec3::new(left, top, far);
  let far_right_down = Vec3::new(right, bottom, far);
  let far_right_top = Vec3::new(right, top, far);

  [
    [near_left_down, near_left_top],
    [near_right_down, near_right_top],
    [near_left_down, near_right_down],
    [near_left_top, near_right_top],
    //
    [far_left_down, far_left_top],
    [far_right_down, far_right_top],
    [far_left_down, far_right_down],
    [far_left_top, far_right_top],
    //
    [near_left_down, far_left_down],
    [near_left_top, far_left_top],
    [near_right_down, far_right_down],
    [near_right_top, far_right_top],
  ]
}
