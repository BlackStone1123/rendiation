use crate::*;

pub struct ShaderFutureMap<F, T> {
  pub upstream: T,
  pub map: F,
}

impl<F, T, O> ShaderFuture for ShaderFutureMap<F, T>
where
  T: ShaderFuture,
  F: FnOnce(T::Output, &mut DeviceTaskSystemPollCtx) -> O + Copy + 'static,
  O: ShaderAbstractRightValue + Default,
{
  type Output = O;
  type Invocation = ShaderFutureMapState<T::Invocation, F>;

  fn required_poll_count(&self) -> usize {
    self.upstream.required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    ShaderFutureMapState {
      upstream: self.upstream.build_poll(ctx),
      map: self.map,
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.upstream.bind_input(builder)
  }
}

pub struct ShaderFutureMapState<T, F> {
  upstream: T,
  map: F,
}

impl<T, F, O> ShaderFutureInvocation for ShaderFutureMapState<T, F>
where
  T: ShaderFutureInvocation,
  F: FnOnce(T::Output, &mut DeviceTaskSystemPollCtx) -> O + 'static + Copy,
  O: Default + ShaderAbstractRightValue,
{
  type Output = O;
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<O> {
    let r = self.upstream.device_poll(ctx);
    let output = LocalLeftValueBuilder.create_left_value(O::default());
    if_by(r.is_resolved(), || {
      let o = (self.map)(r.payload, ctx);
      output.abstract_store(o);
    });

    (r.resolved, output.abstract_load()).into()
  }
}
