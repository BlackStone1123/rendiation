use std::rc::Rc;

use crate::*;
use rendiation_webgpu::GPU;
use winit::event::*;

pub struct EventCtx<'a> {
  pub event: &'a winit::event::Event<'a, ()>,
  pub states: &'a WindowState,
  pub gpu: Rc<GPU>,
}

pub struct EventHandler<T, X> {
  state: X,
  handler: Box<dyn Fn(&mut T)>,
}

impl<T, X: Default> EventHandler<T, X> {
  pub fn by(fun: impl Fn(&mut T) + 'static) -> Self {
    Self {
      state: Default::default(),
      handler: Box::new(fun),
    }
  }
}

impl<T, X: EventHandlerImpl<C>, C: Component<T>> ComponentAbility<T, C> for EventHandler<T, X> {
  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {
    if self.state.downcast_event(event, inner) {
      (self.handler)(model)
    }
    inner.event(model, event);
  }
}

impl<T, X, C: Presentable> PresentableAbility<C> for EventHandler<T, X> {
  fn render(&self, builder: &mut PresentationBuilder, inner: &C) {
    inner.render(builder);
  }
}

impl<T, X, C: LayoutAble> LayoutAbility<C> for EventHandler<T, X> {
  fn layout(
    &mut self,
    constraint: LayoutConstraint,
    ctx: &mut LayoutCtx,
    inner: &mut C,
  ) -> LayoutSize {
    inner.layout(constraint, ctx)
  }

  fn set_position(&mut self, position: UIPosition, inner: &mut C) {
    inner.set_position(position)
  }
}

impl<T, X, C: HotAreaProvider> HotAreaPassBehavior<C> for EventHandler<T, X> {
  fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
    inner.is_point_in(point)
  }
}

pub trait EventHandlerImpl<C> {
  fn downcast_event(&self, event: &mut EventCtx, inner: &C) -> bool;
}

pub struct Click {
  mouse_down: bool,
}
impl Default for Click {
  fn default() -> Self {
    Self { mouse_down: false }
  }
}

pub type ClickHandler<T> = EventHandler<T, Click>;

impl<C: HotAreaProvider> EventHandlerImpl<C> for Click {
  fn downcast_event(&self, event: &mut EventCtx, inner: &C) -> bool {
    if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event.event) {
      if inner.is_point_in(event.states.mouse_position) {
        return true;
      }
    }
    false
  }
}

pub trait HotAreaProvider {
  fn is_point_in(&self, point: UIPosition) -> bool;
}

fn window_event<'a>(event: &'a Event<()>) -> Option<&'a WindowEvent<'a>> {
  match event {
    Event::WindowEvent { event, .. } => Some(event),
    _ => None,
  }
}

fn mouse(event: &Event<()>) -> Option<(MouseButton, ElementState)> {
  window_event(event).and_then(|e| match e {
    WindowEvent::MouseInput { state, button, .. } => Some((*button, *state)),
    _ => None,
  })
}
