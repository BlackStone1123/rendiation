#![feature(impl_trait_in_assoc_type)]

use std::{
  ops::Deref,
  sync::atomic::{AtomicU64, Ordering},
};

use dyn_downcast::*;
use futures::{Future, Stream, StreamExt};
use heap_tools::Counted;
use incremental::*;
use reactive::*;

mod shared;
pub use shared::*;

mod group;
pub use group::*;

static GLOBAL_ID: AtomicU64 = AtomicU64::new(0);

pub fn alloc_global_res_id() -> u64 {
  GLOBAL_ID.fetch_add(1, Ordering::Relaxed)
}

pub struct IncrementalSignal<T: IncrementalBase> {
  id: u64,
  inner: T,
  pub delta_source: EventSource<T::Delta>,
  _counting: Counted<Self>,
}

impl<T: IncrementalBase> AsRef<T> for IncrementalSignal<T> {
  fn as_ref(&self) -> &T {
    &self.inner
  }
}

impl<T: IncrementalBase> From<T> for IncrementalSignal<T> {
  fn from(inner: T) -> Self {
    Self::new(inner)
  }
}

trait ModifyIdentityDelta<T: ApplicableIncremental> {
  fn apply(self, target: &mut IncrementalSignal<T>);
}

impl<T, X> ModifyIdentityDelta<T> for X
where
  T: ApplicableIncremental<Delta = X>,
{
  fn apply(self, target: &mut IncrementalSignal<T>) {
    target.mutate(|mut m| {
      m.modify(self);
    })
  }
}

/// A globally marked item, marked by a globally incremental u64 flag
pub trait GlobalIdentified {
  fn guid(&self) -> u64;
}
define_dyn_trait_downcaster_static!(GlobalIdentified);

impl<T: IncrementalBase> GlobalIdentified for IncrementalSignal<T> {
  fn guid(&self) -> u64 {
    self.id
  }
}
impl<T: IncrementalBase> AsRef<dyn GlobalIdentified> for IncrementalSignal<T> {
  fn as_ref(&self) -> &(dyn GlobalIdentified + 'static) {
    self
  }
}
impl<T: IncrementalBase> AsMut<dyn GlobalIdentified> for IncrementalSignal<T> {
  fn as_mut(&mut self) -> &mut (dyn GlobalIdentified + 'static) {
    self
  }
}

impl<T: IncrementalBase> IncrementalSignal<T> {
  pub fn new(inner: T) -> Self {
    Self {
      inner,
      id: alloc_global_res_id(),
      delta_source: Default::default(),
      _counting: Default::default(),
    }
  }

  pub fn mutate_unchecked<R>(&mut self, mutator: impl FnOnce(&mut T) -> R) -> R {
    mutator(&mut self.inner)
  }

  pub fn mutate<R>(&mut self, mutator: impl FnOnce(Mutating<T>) -> R) -> R {
    self.mutate_with(mutator, |_| {})
  }

  pub fn mutate_with<R>(
    &mut self,
    mutator: impl FnOnce(Mutating<T>) -> R,
    mut extra_collector: impl FnMut(T::Delta),
  ) -> R {
    let data = &mut self.inner;
    let dispatcher = &self.delta_source;
    mutator(Mutating {
      inner: data,
      collector: &mut |delta| {
        dispatcher.emit(delta);
        extra_collector(delta.clone())
      },
    })
  }

  pub fn unbound_listen_by<U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl Stream<Item = U>
  where
    U: Send + Sync + 'static,
  {
    self.listen_by::<U, _, _>(mapper, &DefaultUnboundChannel)
  }

  pub fn single_listen_by<U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl Stream<Item = U>
  where
    U: Send + Sync + 'static,
  {
    self.listen_by::<U, _, _>(mapper, &DefaultSingleValueChannel)
  }

  pub fn listen_by<N, C, U>(
    &self,
    mut mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
    channel_builder: &C,
  ) -> impl Stream<Item = N>
  where
    U: Send + Sync + 'static,
    C: ChannelLike<U, Message = N>,
  {
    let (sender, receiver) = channel_builder.build();

    mapper(MaybeDeltaRef::All(self), &|mapped| {
      C::send(&sender, mapped);
    });

    let remove_token = self.delta_source.on(move |v| {
      mapper(MaybeDeltaRef::Delta(v), &|mapped| {
        C::send(&sender, mapped);
      });
      C::is_closed(&sender)
    });

    let dropper = EventSourceDropper::new(remove_token, self.delta_source.make_weak());
    EventSourceStream::new(dropper, receiver)
  }

  pub fn create_drop(&self) -> impl Future<Output = ()> {
    let mut s = self.single_listen_by(no_change);

    Box::pin(async move {
      loop {
        if s.next().await.is_none() {
          break;
        }
      }
    })
  }
}

impl<T: Default + IncrementalBase> Default for IncrementalSignal<T> {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl<T: IncrementalBase> std::ops::Deref for IncrementalSignal<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

/// An wrapper struct that prevent outside directly accessing the mutable T, but have to modify it
/// through the explicit delta type. When modifying, the delta maybe checked if is really valid by
/// diffing, and the change will be collect by a internal collector
pub struct Mutating<'a, T: IncrementalBase> {
  inner: &'a mut T,
  collector: &'a mut dyn FnMut(&T::Delta),
}

impl<'a, T: IncrementalBase> Deref for Mutating<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.inner
  }
}

impl<'a, T: ApplicableIncremental> Mutating<'a, T> {
  pub fn modify(&mut self, delta: T::Delta) {
    if self.inner.should_apply_hint(&delta) {
      (self.collector)(&delta);
      self.inner.apply(delta).unwrap()
    }
  }
}

impl<'a, T: IncrementalBase> Mutating<'a, T> {
  /// # Safety
  /// the mutation should be record manually, and will not triggered in the collector
  pub unsafe fn get_mut_ref(&mut self) -> &mut T {
    self.inner
  }

  /// # Safety
  /// the mutation will be not apply on original data but only triggered in the collector
  pub unsafe fn trigger_change_but_not_apply(&mut self, delta: T::Delta) {
    (self.collector)(&delta);
  }
}
