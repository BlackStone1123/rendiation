use crate::rinecraft::Rinecraft;
use rendiation_math::*;
use rendiation_render_entity::{raycaster::Raycaster, PerspectiveCamera};
use rendium::*;

impl Rinecraft {
  pub fn use_orbit_controller(&mut self) {
    self
      .window_session
      .add_listener(EventType::MouseMotion, |event_ctx| {
        let state = &mut event_ctx.state;
        if state.window_state.is_left_mouse_down {
          state.orbit_controller.rotate(Vec2::new(
            -state.window_state.mouse_motion.0,
            -state.window_state.mouse_motion.1,
          ))
        }
        if state.window_state.is_right_mouse_down {
          state.orbit_controller.pan(Vec2::new(
            -state.window_state.mouse_motion.0,
            -state.window_state.mouse_motion.1,
          ))
        }
      });

    self
      .window_session
      .add_listener(EventType::MouseWheel, |event_ctx| {
        let state = &mut event_ctx.state;
        let delta = state.window_state.mouse_wheel_delta.1;
        state.orbit_controller.zoom(1.0 - delta * 0.1);
      });
  }

  pub fn use_fps_controller(&mut self) {
    self.window_session.add_listener_raw(|event_ctx| {
      let state = &mut event_ctx.state;
      // match event_ctx.event {

      // }
    });
  }

  pub fn init_world(&mut self) {
    self
      .window_session
      .add_listener(EventType::MouseDown, |event_ctx| {
        let state = &mut event_ctx.state;
        let x_ratio = state.window_state.mouse_position.0 / state.window_state.size.0;
        let y_ratio = 1. - state.window_state.mouse_position.1 / state.window_state.size.1;
        assert!(x_ratio <= 1.);
        assert!(y_ratio <= 1.);
        let ray = state
          .scene
          .get_active_camera_mut_downcast::<PerspectiveCamera>()
          .create_screen_ray(Vec2::new(x_ratio, y_ratio));
        state.world.delete_block_by_ray(&ray);
      });
  }
}
