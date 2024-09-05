use rendiation_shader_api::{bytes_of, Pod};

use crate::*;

type StorageBufferImpl<T> = GrowableDirectQueueUpdateBuffer<StorageBufferReadOnlyDataView<[T]>>;

/// group of(Rxc<id, T fieldChange>) =maintain=> storage buffer <T>
pub struct ReactiveStorageBufferContainer<T: Std430> {
  inner: MultiUpdateContainer<StorageBufferImpl<T>>,
  current_size: u32,
  // resize is fully decided by user, and it's user's responsibility to avoid frequently resizing
  resizer: Box<dyn Stream<Item = u32> + Unpin>,
}

fn make_init_size<T: Std430>(size: usize) -> StorageBufferInit<'static, [T]> {
  let bytes = size * std::mem::size_of::<T>();
  let bytes = std::num::NonZeroU64::new(bytes as u64).unwrap();
  StorageBufferInit::<[T]>::Zeroed(bytes)
}

impl<T: Std430> ReactiveStorageBufferContainer<T> {
  pub fn new(gpu_ctx: GPU, max: impl Stream<Item = u32> + Unpin + 'static) -> Self {
    let init_capacity = 128;
    let data =
      StorageBufferReadOnlyDataView::create_by(&gpu_ctx.device, make_init_size(init_capacity));
    let data = create_growable_buffer(&gpu_ctx, data, u32::MAX);

    let inner = MultiUpdateContainer::new(data);

    Self {
      inner,
      current_size: init_capacity as u32,
      resizer: Box::new(max),
    }
  }

  pub fn poll_update(&mut self, cx: &mut Context) -> StorageBufferReadOnlyDataView<[T]> {
    if let Poll::Ready(Some(max_idx)) = self.resizer.poll_next_unpin(cx) {
      // resize target
      // todo shrink check?
      if max_idx > self.current_size {
        self.current_size = max_idx;
        self.inner.resize(max_idx);
      }
    }
    self.inner.poll_update(cx);
    self.inner.target.gpu().clone()
  }

  pub fn with_source<K: CKey + LinearIdentification, V: CValue + Pod>(
    mut self,
    source: impl ReactiveCollection<K, V>,
    field_offset: usize,
  ) -> Self {
    let updater = CollectionToStorageBufferUpdater {
      field_offset: field_offset as u32,
      upstream: source,
      phantom: PhantomData,
    };

    self.inner.add_source(updater);
    self
  }
}

struct CollectionToStorageBufferUpdater<T, K, V> {
  field_offset: u32,
  upstream: T,
  phantom: PhantomData<(K, V)>,
}

impl<T, C, K, V> CollectionUpdate<StorageBufferImpl<T>>
  for CollectionToStorageBufferUpdater<C, K, V>
where
  T: Std430,
  V: CValue + Pod,
  K: CKey + LinearIdentification,
  C: ReactiveCollection<K, V>,
{
  fn update_target(&mut self, target: &mut StorageBufferImpl<T>, cx: &mut Context) {
    let (changes, _) = self.upstream.poll_changes(cx);
    for (k, v) in changes.iter_key_value() {
      let index = k.alloc_index();

      match v {
        ValueChange::Delta(v, _) => unsafe {
          target
            .set_value_sub_bytes(index, self.field_offset as usize, bytes_of(&v))
            .unwrap();
        },
        ValueChange::Remove(_) => {
          // we could do clear in debug mode
        }
      }
    }
  }
}

impl<T: Std430> BindableResourceProvider for ReactiveStorageBufferContainer<T> {
  fn get_bindable(&self) -> BindingResourceOwned {
    self.inner.gpu().get_bindable()
  }
}
impl<T: Std430> CacheAbleBindingSource for ReactiveStorageBufferContainer<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.inner.gpu().get_binding_build_source()
  }
}
impl<T: Std430> BindableResourceView for ReactiveStorageBufferContainer<T> {
  fn as_bindable(&self) -> rendiation_webgpu::BindingResource {
    self.inner.gpu().as_bindable()
  }
}
