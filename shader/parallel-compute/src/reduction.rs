use crate::*;

struct WorkGroupReductionCompute<T, S> {
  workgroup_size: u32,
  reduction_logic: PhantomData<S>,
  upstream: Box<dyn DeviceInvocationComponent<Node<T>>>,
}

impl<T: 'static, S: 'static> ShaderHashProvider for WorkGroupReductionCompute<T, S> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.workgroup_size.hash(hasher);
    self.upstream.hash_pipeline_with_type_info(hasher)
  }
  shader_hash_type_id! {}
}

impl<T, S> DeviceInvocationComponent<Node<T>> for WorkGroupReductionCompute<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceMonoidLogic<Data = T> + 'static,
{
  fn requested_workgroup_size(&self) -> Option<u32> {
    Some(self.workgroup_size)
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    let source = self.upstream.build_shader(builder);

    let r = builder.entry_by(|cx| {
      let (input, valid) = source.invocation_logic(cx.global_invocation_id());

      let input = valid.select(input, S::identity());

      let shared = cx.define_workgroup_shared_var_host_size_array::<T>(self.workgroup_size);

      let local_id = cx.local_invocation_id().x();

      shared.index(local_id).store(input);

      let iter = self.workgroup_size.ilog2();

      iter.into_shader_iter().for_each(|i, _| {
        cx.workgroup_barrier();

        let stride = val(1) << (val(iter) - i - val(1));

        if_by(local_id.less_than(stride), || {
          let a = shared.index(local_id).load();
          let b = shared.index(local_id + stride).load();
          let combined = S::combine(a, b);
          shared.index(local_id).store(combined);
        });

        cx.workgroup_barrier();
      });

      let result = local_id
        .equals(0)
        .select_branched(|| shared.index(0).load(), || S::identity());

      (result, local_id.equals(0))
    });

    source.get_size_into_adhoc(r).into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.upstream.bind_input(builder)
  }

  fn work_size(&self) -> Option<u32> {
    self.upstream.work_size()
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct WorkGroupReduction<T, S> {
  pub workgroup_size: u32,
  pub reduction_logic: PhantomData<S>,
  pub upstream: Box<dyn DeviceParallelComputeIO<T>>,
}

impl<T, S> DeviceParallelCompute<Node<T>> for WorkGroupReduction<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceMonoidLogic<Data = T> + 'static,
{
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    Box::new(WorkGroupReductionCompute::<T, S> {
      workgroup_size: self.workgroup_size,
      upstream: self.upstream.execute_and_expose(cx),
      reduction_logic: Default::default(),
    })
  }

  fn result_size(&self) -> u32 {
    self.upstream.result_size()
  }
}

impl<T, S> DeviceParallelComputeIO<T> for WorkGroupReduction<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceMonoidLogic<Data = T> + 'static,
{
  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> StorageBufferReadOnlyDataView<[T]>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    let workgroup_size = self.workgroup_size;
    custom_write_into_storage_buffer(self, cx, move |global_id| global_id / val(workgroup_size))
  }
}

#[pollster::test]
async fn test1() {
  let input = vec![1_u32; 8];

  let expect = vec![4, 0, 0, 0, 4, 0, 0, 0];

  let workgroup_size = 4;

  input
    .workgroup_scope_reduction::<AdditionMonoid<_>>(workgroup_size)
    .single_run_test(&expect)
    .await
}

#[pollster::test]
async fn test2() {
  let input = vec![1_u32; 70];

  let expect = vec![70];

  let workgroup_size = 32;

  input
    .segmented_reduction::<AdditionMonoid<_>>(workgroup_size, workgroup_size)
    .single_run_test(&expect)
    .await
}
