use crate::*;
pub struct TaskFuture<T>(usize, PhantomData<T>);

impl<T> TaskFuture<T> {
  pub fn new(id: usize) -> Self {
    Self(id, PhantomData)
  }
}

pub const UN_INIT_TASK_HANDLE: u32 = u32::MAX - 1;
pub const RESOLVED_TASK_HANDLE: u32 = u32::MAX;

impl<T> ShaderFuture for TaskFuture<T>
where
  T: ShaderSizedValueNodeType + Default + Copy,
{
  type Output = Node<T>;
  type Invocation = TaskFutureInvocation<T>;

  fn required_poll_count(&self) -> usize {
    1
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    TaskFutureInvocation {
      task_handle: ctx
        .state_builder
        .create_or_reconstruct_inline_state_with_default(UN_INIT_TASK_HANDLE),
      spawner: ctx.get_or_create_task_group_instance(self.0),
      phantom: PhantomData,
    }
  }

  fn bind_input(&self, _: &mut DeviceTaskSystemBindCtx) {
    // all task binding is handled up front
  }
  fn reset(&mut self, _: &mut DeviceParallelComputeCtx, _: u32) {}
}

pub struct TaskFutureInvocation<T> {
  pub spawner: TaskGroupDeviceInvocationInstanceLateResolved,
  task_handle: BoxedShaderLoadStore<Node<u32>>,
  phantom: PhantomData<T>,
}

impl<T> ShaderFutureInvocation for TaskFutureInvocation<T>
where
  T: ShaderSizedValueNodeType + Default + Copy,
{
  type Output = Node<T>;

  fn device_poll(&self, _ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    let output = LocalLeftValueBuilder.create_left_value(zeroed_val());

    let task_handle = self.task_handle.abstract_load();

    // this check maybe not needed
    let task_has_already_resolved = task_handle.equals(RESOLVED_TASK_HANDLE);
    let task_not_allocated = task_handle.equals(UN_INIT_TASK_HANDLE);

    let result = task_has_already_resolved.make_local_var();

    // once task resolved, it can not be polled again because the states is deallocated.
    // also, should skip simply because task not allocated at all.
    let should_poll = task_has_already_resolved
      .not()
      .and(task_not_allocated.not());

    if_by(should_poll, || {
      let resolved = self.spawner.poll_task::<T>(task_handle, |r| {
        output.abstract_store(r);
        self.task_handle.abstract_store(val(RESOLVED_TASK_HANDLE));
      });
      result.store(resolved);
    });

    (result.load(), output.abstract_load()).into()
  }
}

pub struct TaskFutureInvocationRightValue {
  pub task_handle: Node<u32>,
}

impl<T> ShaderAbstractLeftValue for TaskFutureInvocation<T> {
  type RightValue = TaskFutureInvocationRightValue;

  fn abstract_load(&self) -> Self::RightValue {
    TaskFutureInvocationRightValue {
      task_handle: self.task_handle.abstract_load(),
    }
  }

  fn abstract_store(&self, payload: Self::RightValue) {
    self.task_handle.abstract_store(payload.task_handle);
  }
}
