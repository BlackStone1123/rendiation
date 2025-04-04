use crate::*;

pub struct CombinedBufferAllocatorInternal {
  label: String,
  recording_bind_index_buffer: StorageBufferDataView<[u32]>,
  current_recording_count: u32,
  current_shader_recording_count: u32,
  gpu: GPU,
  buffer: Option<GPUBufferResourceView>,
  usage: BufferUsages,
  buffer_need_rebuild: bool,
  sub_buffer_u32_size_requirements: Vec<u32>,
  sub_buffer_allocation_u32_offset: Vec<u32>,
  pub(crate) layout: StructLayoutTarget,
  // use none for none atomic heap
  atomic: Option<ShaderAtomicValueType>,
  enable_debug_log: bool,
}

const MAX_BINDING_COUNT: usize = 1024;

impl CombinedBufferAllocatorInternal {
  /// label must unique across binding
  pub fn new(
    gpu: &GPU,
    label: impl Into<String>,
    usage: BufferUsages,
    layout: StructLayoutTarget,
    atomic: Option<ShaderAtomicValueType>,
  ) -> Self {
    Self {
      label: label.into(),
      buffer: None,
      buffer_need_rebuild: true,
      sub_buffer_u32_size_requirements: Default::default(),
      sub_buffer_allocation_u32_offset: Default::default(),
      recording_bind_index_buffer: create_gpu_read_write_storage::<[u32]>(
        ZeroedArrayByArrayLength(MAX_BINDING_COUNT),
        &gpu.device,
      ),
      current_recording_count: 0,
      current_shader_recording_count: 0,
      gpu: gpu.clone(),
      usage,
      layout,
      atomic,
      enable_debug_log: false,
    }
  }
  pub fn expect_buffer(&self) -> &GPUBufferResourceView {
    let err = "merged buffer not yet build";
    assert!(!self.buffer_need_rebuild, "{err}");
    self.buffer.as_ref().expect(err)
  }

  pub fn allocate(&mut self, sub_buffer_u32_size: u32) -> usize {
    self.buffer_need_rebuild = true;
    let buffer_index = self.sub_buffer_u32_size_requirements.len();
    self
      .sub_buffer_u32_size_requirements
      .push(sub_buffer_u32_size);

    buffer_index
  }

  pub fn rebuild(&mut self) {
    let gpu = &self.gpu;
    if !self.buffer_need_rebuild && self.buffer.is_some() {
      return;
    }

    // the sub buffer must be aligned to device limitation because use may directly
    // use the sub buffer as the storage/uniform binding
    let limits = &gpu.info.supported_limits;
    // for simplicity we use the maximum alignment, they are same on my machine.
    let bind_alignment_requirement_in_u32 = limits
      .min_storage_buffer_offset_alignment
      .max(limits.min_uniform_buffer_offset_alignment)
      / 4;

    let sub_buffer_count = self.sub_buffer_u32_size_requirements.len() as u32;
    let header_size = sub_buffer_count * 2 + 1;

    let mut sub_buffer_allocation_u32_offset = Vec::with_capacity(sub_buffer_count as usize);
    let mut used_buffer_size_in_u32 = header_size;

    for sub_buffer_size in &self.sub_buffer_u32_size_requirements {
      // add padding
      used_buffer_size_in_u32 += align_offset(
        used_buffer_size_in_u32 as usize,
        bind_alignment_requirement_in_u32 as usize,
      ) as u32;
      sub_buffer_allocation_u32_offset.push(used_buffer_size_in_u32);
      used_buffer_size_in_u32 += *sub_buffer_size;
    }

    let full_size_requirement_in_u32 = used_buffer_size_in_u32;

    let buffer = {
      let usage = self.usage | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
      let init_size = NonZeroU64::new(full_size_requirement_in_u32 as u64 * 4).unwrap();
      let init = BufferInit::Zeroed(init_size);
      let desc = GPUBufferDescriptor {
        size: init.size(),
        usage,
      };

      let buffer = GPUBuffer::create(&gpu.device, init, usage);
      let buffer = GPUBufferResource::create_with_raw(buffer, desc, &gpu.device);
      buffer.create_default_view()
    };

    // write header
    let new_buffer = buffer.resource.gpu();
    gpu
      .queue
      .write_buffer(new_buffer, 0, bytes_of(&sub_buffer_count));

    let offsets = cast_slice(&sub_buffer_allocation_u32_offset);
    gpu.queue.write_buffer(new_buffer, 4, offsets);
    let sizes = cast_slice(&self.sub_buffer_u32_size_requirements);
    gpu
      .queue
      .write_buffer(new_buffer, 4 + sub_buffer_count as u64 * 4, sizes);

    // old data movement
    if let Some(old_buffer) = &self.buffer {
      let mut encoder = gpu.create_encoder();
      for (i, source_offset) in self.sub_buffer_allocation_u32_offset.iter().enumerate() {
        let size = self.sub_buffer_u32_size_requirements[i];
        let new_offset = sub_buffer_allocation_u32_offset[i];
        encoder.copy_buffer_to_buffer(
          old_buffer.resource.gpu(),
          (source_offset * 4) as u64,
          new_buffer,
          (new_offset * 4) as u64,
          (size * 4) as u64,
        );
      }
      gpu.submit_encoder(encoder);
    }

    self.buffer = Some(buffer);
    self.sub_buffer_allocation_u32_offset = sub_buffer_allocation_u32_offset;
    self.buffer_need_rebuild = false;
  }

  pub fn resize(&mut self, index: usize, new_u32_size: u32) {
    self.sub_buffer_u32_size_requirements[index] = new_u32_size;
    self.buffer_need_rebuild = true;
  }

  pub fn write_content(&mut self, index: usize, content: &[u8]) {
    assert!(!self.buffer_need_rebuild);
    let buffer = self.expect_buffer();
    let offset = self.sub_buffer_allocation_u32_offset[index];
    let offset = (offset * 4) as u64;
    self
      .gpu
      .queue
      .write_buffer(buffer.resource.gpu(), offset, content);
  }

  pub fn get_sub_gpu_buffer_view(&self, index: usize) -> GPUBufferResourceView {
    assert!(!self.buffer_need_rebuild);
    let buffer = self.expect_buffer().clone();

    let offset = self.sub_buffer_allocation_u32_offset[index] as u64;
    let offset = offset * 4;
    let size = self.sub_buffer_u32_size_requirements[index] as u64 * 4;
    let range = GPUBufferViewRange {
      offset,
      size: Some(NonZeroU64::new(size).unwrap()),
    };

    buffer.resource.create_view(range)
  }

  pub fn bind_shader_storage<T: ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &mut self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> ShaderPtrOf<T> {
    let ptr = self.bind_shader_impl(bind_builder, registry, T::maybe_unsized_ty());
    T::create_view_from_raw_ptr(ptr)
  }

  pub fn bind_shader_uniform<T: ShaderSizedValueNodeType + Std140>(
    &mut self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> ShaderReadonlyPtrOf<T> {
    let ptr = self.bind_shader_impl(bind_builder, registry, T::maybe_unsized_ty());
    T::create_readonly_view_from_raw_ptr(ptr)
  }

  #[inline(never)]
  pub fn bind_shader_impl(
    &mut self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
    ty_desc: MaybeUnsizedValueType,
  ) -> BoxedShaderPtr {
    let label = &self.label;

    #[derive(Clone)]
    struct ShaderMeta {
      pub meta: Arc<RwLock<ShaderU32StructMetaData>>,
      pub array: U32HeapHeapSource,
      pub bind_index_array: ShaderPtrOf<[u32]>,
    }

    let (_, array) = registry
      .dynamic_anything
      .raw_entry_mut()
      .from_key(label)
      .or_insert_with(|| {
        if self.enable_debug_log {
          println!("bind shader <{}>", self.label);
        }

        let heap_ty = if let Some(a_ty) = self.atomic {
          match a_ty {
            ShaderAtomicValueType::I32 => <[DeviceAtomic<i32>]>::ty(),
            ShaderAtomicValueType::U32 => <[DeviceAtomic<u32>]>::ty(),
          }
        } else {
          <[u32]>::ty()
        };

        let handle = bind_builder
          .binding_dyn(ShaderBindingDescriptor {
            should_as_storage_buffer_if_is_buffer_like: true,
            ty: heap_ty,
            writeable_if_storage: true,
          })
          .using();

        let ptr = Box::new(handle);
        let array = if let Some(a_ty) = self.atomic {
          match a_ty {
            ShaderAtomicValueType::I32 => {
              U32HeapHeapSource::AtomicI32(<[DeviceAtomic<i32>]>::create_view_from_raw_ptr(ptr))
            }
            ShaderAtomicValueType::U32 => {
              U32HeapHeapSource::AtomicU32(<[DeviceAtomic<u32>]>::create_view_from_raw_ptr(ptr))
            }
          }
        } else {
          U32HeapHeapSource::Common(<[u32]>::create_view_from_raw_ptr(ptr))
        };

        let meta = ShaderMeta {
          meta: Arc::new(RwLock::new(ShaderU32StructMetaData::new(self.layout))),
          array,
          bind_index_array: bind_builder.bind_by(&self.recording_bind_index_buffer),
        };
        self.current_shader_recording_count = 0;

        (label.to_string(), Arc::new(meta))
      });

    let ShaderMeta {
      meta,
      array,
      bind_index_array,
    } = array.downcast_ref::<ShaderMeta>().unwrap().clone();

    // todo, should we put it at allocation time?
    meta.write().register_ty(&ty_desc);

    let buffer_bind_index = bind_index_array
      .index(self.current_shader_recording_count)
      .load();
    let offset = array.bitcast_read_u32_at(buffer_bind_index + val(1));

    self.current_shader_recording_count += 1;

    let ptr = U32HeapPtrWithType {
      ptr: U32HeapPtr { array, offset },
      ty: ty_desc.into_shader_single_ty(),
      bind_index: buffer_bind_index,
      meta,
    };
    Box::new(ptr)
  }

  pub fn bind_pass(&mut self, bind_builder: &mut BindingBuilder, index: usize) {
    let buffer = self.expect_buffer();
    let bounded = bind_builder
      .iter_groups()
      .flat_map(|g| g.iter_bounded())
      .any(|res| res.view_id == buffer.guid);

    if !bounded {
      if self.enable_debug_log {
        println!("bind res <{}>", self.label);
      }
      bind_builder.bind_dyn(buffer.get_binding_build_source());
      // new binding occurs, refresh binding index buffer
      self.recording_bind_index_buffer = create_gpu_read_write_storage::<[u32]>(
        ZeroedArrayByArrayLength(MAX_BINDING_COUNT),
        &self.gpu.device,
      );
      self.current_recording_count = 0;
      bind_builder.bind(&self.recording_bind_index_buffer);
    }
    self.gpu.queue.write_buffer(
      self.recording_bind_index_buffer.buffer.gpu(),
      self.current_recording_count as u64 * 4,
      bytes_of(&(index as u32)),
    );

    self.current_recording_count += 1;
  }
}
