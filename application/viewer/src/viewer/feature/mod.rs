mod camera_control;
pub use camera_control::*;
mod gizmo_bridge;
pub use gizmo_bridge::*;
mod fit_camera_view;
pub use fit_camera_view::*;
mod pick_scene;
pub use pick_scene::*;

use crate::*;

pub fn core_viewer_features<V: Widget + 'static>(
  content_logic: impl Fn(&mut DynCx) -> V + 'static,
) -> impl Fn(&mut DynCx) -> Box<dyn Widget> {
  move |cx| {
    let gizmo = StateCxCreateOnce::new(GizmoBridge::new);
    Box::new(
      WidgetGroup::default()
        .with_child(SceneOrbitCameraControl::default())
        .with_child(PickScene {
          enable_hit_debug_log: false,
        })
        .with_child(gizmo)
        .with_child(content_logic(cx)),
    )
  }
}
