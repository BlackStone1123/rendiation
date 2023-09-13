use std::{marker::PhantomData, sync::Arc};

use futures::{Future, Stream};

use crate::*;

// we are not extract to the upstream storage crate here for now.
pub struct SourceStorage<T> {
  inner: Vec<(u32, T)>,
  next_id: u32,
}

impl<T> SourceStorage<T> {
  pub fn insert(&mut self, item: T) -> u32 {
    self.next_id += 1;
    self.inner.push((self.next_id, item));
    self.next_id
  }

  pub fn remove(&mut self, handle: u32) {
    let idx = self
      .inner
      .iter()
      .position(|v| v.0 == handle)
      .expect("event source remove failed");
    let _ = self.inner.swap_remove(idx);
  }

  pub fn iter_remove_if(&mut self, f: impl Fn(&mut T) -> bool) {
    let mut idx = 0;
    while idx < self.inner.len() {
      let item = &mut self.inner[idx].1;
      if f(item) {
        self.inner.swap_remove(idx);
      } else {
        idx += 1;
      }
    }
  }
}

impl<T> Default for SourceStorage<T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
      next_id: 0,
    }
  }
}

pub type EventListener<T> = Box<dyn FnMut(&T) -> bool + Send + Sync>;

pub struct Source<T> {
  // return if should remove
  storage: SourceStorage<EventListener<T>>,
}

pub struct RemoveToken<T> {
  handle: u32,
  phantom: PhantomData<T>,
}

impl<T> Clone for RemoveToken<T> {
  fn clone(&self) -> Self {
    Self {
      handle: self.handle,
      phantom: PhantomData,
    }
  }
}
impl<T> Copy for RemoveToken<T> {}

impl<T> Source<T> {
  /// return should be removed from source after emitted
  pub fn on(&mut self, cb: impl FnMut(&T) -> bool + Send + Sync + 'static) -> RemoveToken<T> {
    RemoveToken {
      handle: self.storage.insert(Box::new(cb)),
      phantom: PhantomData,
    }
  }
  pub fn off(&mut self, token: RemoveToken<T>) {
    self.storage.remove(token.handle);
  }

  #[allow(unused_must_use)]
  pub fn emit(&mut self, event: &T) {
    self.storage.iter_remove_if(|f| f(event));
  }
}

impl<T> Default for Source<T> {
  fn default() -> Self {
    Self {
      storage: Default::default(),
    }
  }
}

/// a simple event dispatcher.
pub struct EventSource<T> {
  /// the source is alway need mutable access, so we not use rwlock here
  inner: Arc<Mutex<Source<T>>>,
}

impl<T> Default for EventSource<T> {
  // default not to do any allocation when created
  // as long as no one add listener, no allocation happens
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}

impl<T> Clone for EventSource<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T: 'static> EventSource<T> {
  pub fn make_weak(&self) -> WeakSource<T> {
    WeakSource {
      inner: Arc::downgrade(&self.inner),
    }
  }

  pub fn emit(&self, event: &T) {
    let mut inner = self.inner.lock().unwrap();
    inner.emit(event);
  }

  /// return should be removed from source after emitted
  pub fn on(&self, f: impl FnMut(&T) -> bool + Send + Sync + 'static) -> RemoveToken<T> {
    self.inner.lock().unwrap().on(f)
  }

  pub fn off(&self, token: RemoveToken<T>) {
    self.inner.lock().unwrap().off(token)
  }

  pub fn any_triggered(&self) -> impl futures::Stream<Item = ()> {
    self.single_listen_by(|_| (), |_| ())
  }

  pub fn single_listen(&self) -> impl futures::Stream<Item = T>
  where
    T: Clone + Send + Sync,
  {
    self.single_listen_by(|v| v.clone(), |_| {})
  }

  pub fn unbound_listen(&self) -> impl futures::Stream<Item = T>
  where
    T: Clone + Send + Sync,
  {
    self.unbound_listen_by(|v| v.clone(), |_| {})
  }

  pub fn batch_listen(&self) -> impl futures::Stream<Item = Vec<T>>
  where
    T: Clone + Send + Sync,
  {
    self.batch_listen_by(|v| v.clone(), |_| {})
  }

  pub fn unbound_listen_by<U>(
    &self,
    mapper: impl Fn(&T) -> U + Send + Sync + 'static,
    init: impl FnOnce(&dyn Fn(U)),
  ) -> impl futures::Stream<Item = U>
  where
    U: Send + Sync + 'static,
  {
    self.listen_by::<U, _, _>(mapper, init, &DefaultUnboundChannel)
  }

  pub fn batch_listen_by<U>(
    &self,
    mapper: impl Fn(&T) -> U + Send + Sync + 'static,
    init: impl FnOnce(&dyn Fn(U)),
  ) -> impl futures::Stream<Item = Vec<U>>
  where
    U: Send + Sync + 'static,
  {
    self.listen_by::<Vec<U>, _, _>(mapper, init, &DefaultBatchChannel)
  }

  pub fn single_listen_by<U>(
    &self,
    mapper: impl Fn(&T) -> U + Send + Sync + 'static,
    init: impl FnOnce(&dyn Fn(U)),
  ) -> impl futures::Stream<Item = U> + 'static
  where
    U: Send + Sync + 'static,
  {
    self.listen_by::<U, _, _>(mapper, init, &DefaultSingleValueChannel)
  }

  pub fn listen_by<N, C, U>(
    &self,
    mapper: impl Fn(&T) -> U + Send + Sync + 'static,
    init: impl FnOnce(&dyn Fn(U)),
    channel_builder: &C,
  ) -> impl futures::Stream<Item = N> + 'static
  where
    U: Send + Sync + 'static,
    C: ChannelLike<U, Message = N>,
  {
    let (sender, receiver) = channel_builder.build();
    let init_sends = |to_send| {
      C::send(&sender, to_send);
    };
    init(&init_sends);
    let remove_token = self.on(move |v| {
      C::send(&sender, mapper(v));
      C::is_closed(&sender)
    });
    let dropper = EventSourceDropper::new(remove_token, self.make_weak());
    EventSourceStream::new(dropper, receiver)
  }

  pub fn once_future<R>(
    &mut self,
    f: impl FnOnce(&T) -> R + Send + Sync + 'static,
  ) -> impl Future<Output = R>
  where
    T: Send + Sync,
    R: Send + Sync + 'static,
  {
    use futures::FutureExt;
    let f = Mutex::new(Some(f));
    let f = move |p: &_| f.lock().unwrap().take().map(|f| f(p));
    let any = self.single_listen_by(f, |_| {});
    any.into_future().map(|(r, _)| r.unwrap().unwrap())
  }
}

#[pin_project::pin_project]
pub struct EventSourceStream<T, S> {
  dropper: EventSourceDropper<T>,
  #[pin]
  stream: S,
}

impl<T, S> EventSourceStream<T, S> {
  pub fn new(dropper: EventSourceDropper<T>, stream: S) -> Self {
    Self { dropper, stream }
  }
}

impl<T, S> Stream for EventSourceStream<T, S>
where
  S: Stream,
{
  type Item = S::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.project().stream.poll_next(cx)
  }
}

pub struct EventSourceDropper<T> {
  remove_token: RemoveToken<T>,
  weak: WeakSource<T>,
}

impl<T> EventSourceDropper<T> {
  pub fn new(remove_token: RemoveToken<T>, weak: WeakSource<T>) -> Self {
    Self { remove_token, weak }
  }
}

impl<T> Drop for EventSourceDropper<T> {
  fn drop(&mut self) {
    if let Some(source) = self.weak.inner.upgrade() {
      // it's safe to remove again here (has no effect)
      source.lock().unwrap().off(self.remove_token)
    }
  }
}

pub struct WeakSource<T> {
  inner: std::sync::Weak<Mutex<Source<T>>>,
}

impl<T> Clone for WeakSource<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T> WeakSource<T> {
  pub fn emit(&self, event: &T) -> bool {
    if let Some(e) = self.inner.upgrade() {
      e.lock().unwrap().emit(event);
      true
    } else {
      false
    }
  }
  pub fn is_exist(&self) -> bool {
    self.inner.upgrade().is_some()
  }
}
