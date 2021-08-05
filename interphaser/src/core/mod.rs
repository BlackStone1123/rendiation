mod layout;
pub use layout::*;

mod rendering;
pub use rendering::*;

use crate::*;
use rendiation_webgpu::GPU;
use std::rc::Rc;

pub trait Component<T, S: System = DefaultSystem> {
  fn event(&mut self, _model: &mut T, _event: &mut S::EventCtx<'_>) {}

  fn update(&mut self, _model: &T, _ctx: &mut S::UpdateCtx<'_>) {}
}

pub trait System {
  type EventCtx<'a>;
  type UpdateCtx<'a>;
}

pub struct DefaultSystem {}

impl System for DefaultSystem {
  type EventCtx<'a> = EventCtx<'a>;
  type UpdateCtx<'a> = UpdateCtx;
}

pub struct UpdateCtx {
  pub time_stamp: u64,
  pub layout_changed: bool, // todo private
}

impl UpdateCtx {
  pub fn request_layout(&mut self) {
    self.layout_changed = true;
  }
}

pub struct EventCtx<'a> {
  pub event: &'a winit::event::Event<'a, ()>,
  pub states: &'a WindowState,
  pub gpu: Rc<GPU>,
}

pub trait UIComponent<T>: Component<T> + Presentable + LayoutAble + 'static {}
impl<X, T> UIComponent<T> for X where X: Component<T> + Presentable + LayoutAble + 'static {}
