use crate::*;

impl GPURaytracingPipelineBuilder {
  pub fn compile_task_executor(
    &self,
    device: &GPUDevice,
    init_size: usize,
  ) -> DeviceTaskGraphExecutor {
    let mut executor = DeviceTaskGraphExecutor::new(1, 1);

    let init_pass = todo!();

    // executor.define_task(
    //   BaseDeviceFuture::default(),
    //   || (),
    //   device,
    //   init_size,
    //   init_pass,
    // );

    // for closet in &self.closest_hit_shaders {
    //   executor.define_task(
    //     BaseDeviceFuture::default(),
    //     || (),
    //     device,
    //     init_size,
    //     init_pass,
    //   );
    // }

    // for ray_gen in &self.ray_gen_shaders {
    //   executor.define_task(
    //     BaseDeviceFuture::default(),
    //     || (),
    //     device,
    //     init_size,
    //     init_pass,
    //   );
    // }

    executor
  }
}
