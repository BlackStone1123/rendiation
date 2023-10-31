use std::{marker::PhantomData, sync::Arc};

use fast_hash_collection::{FastHashMap, FastHashSet};
use futures::task::{ArcWake, AtomicWaker};
use parking_lot::{RwLock, RwLockReadGuard};
use storage::IndexKeptVec;

use crate::*;

#[derive(Debug, Clone, Copy)]
pub enum CollectionDelta<K, V> {
  /// here we not impose any delta on
  Delta(K, V),
  Remove(K),
}

impl<K, V> CollectionDelta<K, V> {
  pub fn map<R>(self, mapper: impl FnOnce(&K, V) -> R) -> CollectionDelta<K, R> {
    type Rt<K, R> = CollectionDelta<K, R>;
    match self {
      Self::Remove(k) => Rt::<K, R>::Remove(k),
      Self::Delta(k, d) => {
        let mapped = mapper(&k, d);
        Rt::<K, R>::Delta(k, mapped)
      }
    }
  }

  pub fn value(self) -> Option<V> {
    match self {
      Self::Delta(_, v) => Some(v),
      Self::Remove(_) => None,
    }
  }

  // should we just use struct??
  pub fn key(&self) -> &K {
    match self {
      Self::Remove(k) => k,
      Self::Delta(k, _) => k,
    }
  }
}

pub trait VirtualCollection<K, V> {
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_;

  fn iter_key_value(&self, skip_cache: bool) -> impl Iterator<Item = (K, V)> + '_ {
    let access = self.access(skip_cache);
    self.iter_key(skip_cache).map(move |k| {
      let v = access(&k).expect("iter_key_value provide key but not have valid value");
      (k, v)
    })
  }

  /// Access the current value. we use this scoped api style for fast batch accessing(avoid internal
  /// fragmented locking). the returned V is pass by ownership because we may create data on the
  /// fly.
  ///
  /// If the skip_cache is true, the implementation will not be incremental and will make sure the
  /// access is up to date. If the return is None, it means the value is not exist in the table.
  ///
  /// The implementation should guarantee it's ok to allow  multiple accessor instance exist in same
  /// time. (should only create read guard in underlayer)
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_;
}

/// An abstraction of reactive key-value like virtual container.
///
/// You can imagine this is a data table with the K as the primary key and V as the row of the
/// data(not contains K). In this table, besides getting data, you can also poll it's partial
/// changes.
///
/// Implementation notes:
///
/// This trait maybe could generalize to SignalLike trait:
/// ```rust
/// pub trait Signal<T: IncrementalBase>: Stream<Item = T::Delta> {
///   fn access(&self) -> T;
/// }
/// ```
/// However, this idea has not baked enough. For example, how do we express efficient partial
/// access for large T or container like T? Should we use some accessor associate trait or type as
/// the accessor key? Should we link this type to the T like how we did in Incremental trait?
pub trait ReactiveCollection<K, V>: VirtualCollection<K, V> + Stream + Unpin + 'static {}

/// The data maybe slate if we combine these two trait directly because the visitor maybe not
/// directly access the original source data, but access the cache. This access abstract the
/// internal cache mechanism. Note, even if the polling issued before access, you still can not
/// guaranteed to access the "current" data due to the multi-threaded source mutation. Because of
/// this limitation, user should make sure their downstream consuming logic is timeline insensitive.
///
/// In the future, maybe we could add new sub-trait to enforce the data access is consistent with
/// the polling logic in tradeoff of the potential memory overhead.
impl<T, K, V> ReactiveCollection<K, V> for T
where
  T: VirtualCollection<K, V> + Stream + Unpin + 'static,
  Self::Item: IntoIterator<Item = CollectionDelta<K, V>>,
{
}

/// dynamic version of the above trait
pub trait DynamicVirtualCollection<K, V> {
  fn iter_key_boxed(&self, skip_cache: bool) -> Box<dyn Iterator<Item = K> + '_>;
  fn access_boxed(&self, skip_cache: bool) -> Box<dyn Fn(&K) -> Option<V> + '_>;
}
impl<K, V, T> DynamicVirtualCollection<K, V> for T
where
  Self: ReactiveCollection<K, V>,
  <Self as Stream>::Item: IntoIterator<Item = CollectionDelta<K, V>>,
{
  fn iter_key_boxed(&self, skip_cache: bool) -> Box<dyn Iterator<Item = K> + '_> {
    Box::new(self.iter_key(skip_cache))
  }

  fn access_boxed(&self, skip_cache: bool) -> Box<dyn Fn(&K) -> Option<V> + '_> {
    Box::new(self.access(skip_cache))
  }
}
pub trait DynamicReactiveCollection<K, V>:
  DynamicVirtualCollection<K, V> + Stream<Item = Vec<CollectionDelta<K, V>>> + Unpin
{
}
impl<K, V> VirtualCollection<K, V> for &dyn DynamicReactiveCollection<K, V> {
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_ {
    self.access_boxed(skip_cache)
  }

  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    self.iter_key_boxed(skip_cache)
  }
}

pub trait ReactiveCollectionExt<K, V>: Sized + 'static + ReactiveCollection<K, V>
where
  Self::Item: IntoIterator<Item = CollectionDelta<K, V>>,
{
  /// map map<k, v> to map<k, v2>
  fn collective_map<V2, F: Fn(V) -> V2 + Copy>(self, f: F) -> ReactiveKVMap<Self, F, K, V2> {
    ReactiveKVMap {
      inner: self,
      map: f,
      phantom: PhantomData,
    }
  }

  /// filter map<k, v> by v
  fn collective_filter<F: Fn(V) -> bool>(
    self,
    f: F,
  ) -> ReactiveKVFilter<Self, FilterToMap<V, F>, K, V>
  where
    V: Copy,
  {
    ReactiveKVFilter {
      inner: self,
      checker: fn_map(f),
      k: PhantomData,
    }
  }

  /// filter map<k, v> by v
  fn collective_filter_map<V2, F: Fn(V) -> Option<V2>>(
    self,
    f: F,
  ) -> ReactiveKVFilter<Self, F, K, V> {
    ReactiveKVFilter {
      inner: self,
      checker: f,
      k: PhantomData,
    }
  }

  // /// filter map<k, v> by reactive set<k>
  // fn filter_by_keyset<S: ReactiveCollection<K, ()>>(
  //   self,
  //   set: S,
  // ) -> impl ReactiveCollection<K, V> {
  //   //
  // }

  fn collective_union<V2, Other>(self, other: Other) -> ReactiveKVUnion<Self, Other, K>
  where
    Other: ReactiveCollection<K, V2>,
    Other::Item: IntoIterator<Item = CollectionDelta<K, V2>>,
  {
    ReactiveKVUnion {
      a: self,
      b: other,
      k: PhantomData,
    }
  }

  /// K should not overlap
  fn collective_select<Other>(
    self,
    other: Other,
  ) -> ReactiveKVMap<ReactiveKVUnion<Self, Other, K>, Selector<V>, K, V>
  where
    K: Copy + std::hash::Hash + Eq + 'static,
    Other: ReactiveCollection<K, V>,
    Other::Item: IntoIterator<Item = CollectionDelta<K, V>>,
  {
    self.collective_union(other).collective_map(selector)
  }

  /// K should fully overlap
  fn collective_zip<Other, V2>(
    self,
    other: Other,
  ) -> ReactiveKVMap<ReactiveKVUnion<Self, Other, K>, Zipper<V, V2>, K, (V, V2)>
  where
    K: Copy + std::hash::Hash + Eq + 'static,
    Other: ReactiveCollection<K, V2>,
    Other::Item: IntoIterator<Item = CollectionDelta<K, V2>>,
  {
    self.collective_union(other).collective_map(zipper)
  }

  /// only return overlapped part
  fn collective_intersect<Other, V2>(
    self,
    other: Other,
  ) -> ReactiveKVFilter<
    ReactiveKVUnion<Self, Other, K>,
    IntersectFn<V, V2>,
    K,
    (Option<V>, Option<V2>),
  >
  where
    V: Copy,
    K: Copy + std::hash::Hash + Eq + 'static,
    Other: ReactiveCollection<K, V2>,
    Other::Item: IntoIterator<Item = CollectionDelta<K, V2>>,
  {
    self
      .collective_union(other)
      .collective_filter_map(intersect_fn)
  }

  fn materialize_unordered(self) -> UnorderedMaterializedReactiveCollection<Self, K, V> {
    UnorderedMaterializedReactiveCollection {
      inner: self,
      cache: Default::default(),
    }
  }
  fn materialize_linear(self) -> LinearMaterializedReactiveCollection<Self, V> {
    LinearMaterializedReactiveCollection {
      inner: self,
      cache: Default::default(),
    }
  }
}
impl<T, K, V> ReactiveCollectionExt<K, V> for T
where
  T: Sized + 'static + ReactiveCollection<K, V>,
  Self::Item: IntoIterator<Item = CollectionDelta<K, V>>,
{
}

fn selector<T>((a, b): (Option<T>, Option<T>)) -> T {
  match (a, b) {
    (Some(_), Some(_)) => unreachable!("key set should not overlap"),
    (Some(a), None) => a,
    (None, Some(b)) => b,
    (None, None) => unreachable!("value not selected"),
  }
}

fn zipper<T, U>((a, b): (Option<T>, Option<U>)) -> (T, U) {
  match (a, b) {
    (Some(a), Some(b)) => (a, b),
    _ => unreachable!("value not zipped"),
  }
}

fn intersect_fn<T, U>((a, b): (Option<T>, Option<U>)) -> Option<(T, U)> {
  match (a, b) {
    (Some(a), Some(b)) => Some((a, b)),
    _ => None,
  }
}

type Selector<T> = impl Fn((Option<T>, Option<T>)) -> T;
type Zipper<T, U> = impl Fn((Option<T>, Option<U>)) -> (T, U);
type IntersectFn<T, U> = impl Fn((Option<T>, Option<U>)) -> Option<(T, U)>;
type FilterToMap<T: Copy, F: Fn(T) -> bool> = impl Fn(T) -> Option<T>;
fn fn_map<T: Copy, F: Fn(T) -> bool>(f: F) -> FilterToMap<T, F> {
  move |v| if f(v) { Some(v) } else { None }
}

// pub struct ReactiveKVMapFork<Map> {
//   inner: Arc<RwLock<Map>>,
//   wakers: Arc<WakerBroadcast>,
//   id: u64,
// }

// struct WakerBroadcast {
//   wakers: RwLock<FastHashMap<u64, AtomicWaker>>,
// }
// impl ArcWake for WakerBroadcast {
//   fn wake_by_ref(arc_self: &Arc<Self>) {
//     let wakers = arc_self.wakers.read();
//     for w in wakers.values() {
//       w.wake()
//     }
//   }
// }

// impl<Map> Drop for ReactiveKVMapFork<Map> {
//   fn drop(&mut self) {
//     self.wakers.wakers.write().remove(&self.id);
//   }
// }
// impl<Map> Clone for ReactiveKVMapFork<Map> {
//   fn clone(&self) -> Self {
//     self.wakers.clone().wake();
//     Self {
//       inner: self.inner.clone(),
//       wakers: self.wakers.clone(),
//       id: alloc_global_res_id(),
//     }
//   }
// }

// impl<Map: Stream + Unpin> Stream for ReactiveKVMapFork<Map> {
//   type Item = Map::Item;

//   fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//     // these writes should not deadlock, because we not prefer the concurrency between the table
//     // updates. if we do allow it in the future, just change it to try write or yield pending.

//     {
//       let mut wakers = self.wakers.wakers.write();
//       let waker = wakers.entry(self.id).or_insert_with(Default::default);
//       waker.register(cx.waker());
//     }

//     let waker = futures::task::waker_ref(&self.wakers);
//     let mut cx = std::task::Context::from_waker(&waker);

//     let mut inner = self.inner.write();
//     inner.poll_next_unpin(&mut cx)
//   }
// }

// impl<K, V, Map> VirtualCollection<K, V> for ReactiveKVMapFork<Map>
// where
//   Map: VirtualCollection<K, V> + 'static,
// {
//   fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
//     struct ReactiveKVMapForkRead<'a, Map, I> {
//       _inner: RwLockReadGuard<'a, Map>,
//       inner_iter: I,
//     }

//     impl<'a, Map, I: Iterator> Iterator for ReactiveKVMapForkRead<'a, Map, I> {
//       type Item = I::Item;

//       fn next(&mut self) -> Option<Self::Item> {
//         self.inner_iter.next()
//       }
//     }

//     /// util to get accessor type
//     type IterOf<'a, M: VirtualCollection<K, V> + 'a, K, V> = impl Iterator<Item = K> + 'a;
//     fn get_iter<'a, K, V, M>(map: &M, skip_cache: bool) -> IterOf<M, K, V>
//     where
//       M: VirtualCollection<K, V> + 'a,
//     {
//       map.iter_key(skip_cache)
//     }

//     let inner = self.inner.read();
//     let inner_iter = get_iter(inner.deref(), skip_cache);
//     // safety: read guard is hold by iter, acc's real reference is form the Map
//     let inner_iter: IterOf<'static, Map, K, V> = unsafe { std::mem::transmute(inner_iter) };
//     ReactiveKVMapForkRead {
//       _inner: inner,
//       inner_iter,
//     }
//   }

//   fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_ {
//     let inner = self.inner.read();

//     /// util to get accessor type
//     type AccessorOf<'a, M: VirtualCollection<K, V> + 'a, K, V> = impl Fn(&K) -> Option<V> + 'a;
//     fn get_accessor<'a, K, V, M>(map: &M, skip_cache: bool) -> AccessorOf<M, K, V>
//     where
//       M: VirtualCollection<K, V> + 'a,
//     {
//       map.access(skip_cache)
//     }

//     let acc: AccessorOf<Map, K, V> = get_accessor(inner.deref(), skip_cache);
//     // safety: read guard is hold by closure, acc's real reference is form the Map
//     let acc: AccessorOf<'static, Map, K, V> = unsafe { std::mem::transmute(acc) };
//     move |key| {
//       let _holder = &inner;
//       let acc = &acc;
//       acc(key)
//     }
//   }
// }

#[pin_project::pin_project]
pub struct UnorderedMaterializedReactiveCollection<Map, K, V> {
  #[pin]
  inner: Map,
  cache: FastHashMap<K, V>,
}

impl<Map, K, V> Stream for UnorderedMaterializedReactiveCollection<Map, K, V>
where
  Map: Stream,
  K: std::hash::Hash + Eq,
  V: IncrementalBase<Delta = V>,
  Map::Item: IntoIterator<Item = CollectionDelta<K, V>> + Clone,
{
  type Item = Map::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let r = this.inner.poll_next(cx);
    if let Poll::Ready(Some(changes)) = &r {
      for change in changes.clone().into_iter() {
        match change {
          CollectionDelta::Delta(k, v) => {
            this.cache.insert(k, v);
          }
          CollectionDelta::Remove(k) => {
            // todo, shrink
            this.cache.remove(&k);
          }
        }
      }
    }
    r
  }
}

impl<K, V, Map> VirtualCollection<K, V> for UnorderedMaterializedReactiveCollection<Map, K, V>
where
  Map: VirtualCollection<K, V>,
  K: std::hash::Hash + Eq + Clone,
  V: Clone,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    if skip_cache {
      Box::new(self.inner.iter_key(skip_cache)) as Box<dyn Iterator<Item = K> + '_>
    } else {
      Box::new(self.cache.keys().cloned()) as Box<dyn Iterator<Item = K> + '_>
    }
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_ {
    let inner = self.inner.access(skip_cache);
    move |key| {
      if skip_cache {
        inner(key)
      } else {
        self.cache.get(key).cloned()
      }
    }
  }
}

#[pin_project::pin_project]
pub struct LinearMaterializedReactiveCollection<Map, V> {
  #[pin]
  inner: Map,
  cache: IndexKeptVec<V>,
}

impl<Map, K, V> Stream for LinearMaterializedReactiveCollection<Map, V>
where
  Map: Stream,
  K: LinearIdentification,
  V: IncrementalBase<Delta = V>,
  Map::Item: IntoIterator<Item = CollectionDelta<K, V>> + Clone,
{
  type Item = Map::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let r = this.inner.poll_next(cx);
    if let Poll::Ready(Some(changes)) = &r {
      for change in changes.clone().into_iter() {
        match change {
          CollectionDelta::Delta(k, v) => {
            this.cache.insert(v, k.alloc_index());
          }
          CollectionDelta::Remove(k) => {
            // todo, shrink
            this.cache.remove(k.alloc_index());
          }
        }
      }
    }
    r
  }
}

impl<K, V, Map> VirtualCollection<K, V> for LinearMaterializedReactiveCollection<Map, V>
where
  Map: VirtualCollection<K, V>,
  K: LinearIdentification + 'static,
  V: Clone,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    if skip_cache {
      Box::new(self.inner.iter_key(skip_cache)) as Box<dyn Iterator<Item = K> + '_>
    } else {
      Box::new(self.cache.iter().map(|(k, _)| K::from_alloc_index(k)))
        as Box<dyn Iterator<Item = K> + '_>
    }
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_ {
    let inner = self.inner.access(skip_cache);
    move |key| {
      if skip_cache {
        inner(key)
      } else {
        self.cache.try_get(key.alloc_index()).cloned()
      }
    }
  }
}

#[pin_project::pin_project]
pub struct ReactiveKVMap<T, F, K, V> {
  #[pin]
  inner: T,
  map: F,
  phantom: PhantomData<(K, V)>,
}

impl<T, F, K, V, V2> Stream for ReactiveKVMap<T, F, K, V>
where
  F: Fn(V) -> V2 + Copy + 'static,
  T: Stream,
  T::Item: IntoIterator<Item = CollectionDelta<K, V>>,
{
  type Item = impl IntoIterator<Item = CollectionDelta<K, V2>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let mapper = *this.map;
    this.inner.poll_next(cx).map(move |r| {
      r.map(move |deltas| {
        deltas
          .into_iter()
          .map(move |delta| delta.map(|_, v| mapper(v)))
      })
    })
  }
}

impl<T, F, K, V, V2> VirtualCollection<K, V2> for ReactiveKVMap<T, F, K, V>
where
  F: Fn(V) -> V2 + Copy,
  T: VirtualCollection<K, V>,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    self.inner.iter_key(skip_cache)
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V2> + '_ {
    let inner_getter = self.inner.access(skip_cache);
    move |key| inner_getter(key).map(|v| (self.map)(v))
  }
}

#[pin_project::pin_project]
pub struct ReactiveKVFilter<T, F, K, V> {
  #[pin]
  inner: T,
  checker: F,
  k: PhantomData<(K, V)>,
}

impl<T, F, K, V, V2> Stream for ReactiveKVFilter<T, F, K, V>
where
  F: Fn(V) -> Option<V2> + Copy + 'static,
  T: Stream,
  T::Item: IntoIterator<Item = CollectionDelta<K, V>>,
{
  type Item = impl IntoIterator<Item = CollectionDelta<K, V2>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let checker = *this.checker;
    this.inner.poll_next(cx).map(move |r| {
      r.map(move |deltas| {
        deltas.into_iter().map(move |delta| match delta {
          CollectionDelta::Delta(k, v) => match checker(v) {
            Some(v) => CollectionDelta::Delta(k, v),
            None => CollectionDelta::Remove(k),
          },
          // the Remove variant maybe called many times for given k
          CollectionDelta::Remove(k) => CollectionDelta::Remove(k),
        })
      })
    })
  }
}

impl<T, F, K, V, V2> VirtualCollection<K, V2> for ReactiveKVFilter<T, F, K, V>
where
  F: Fn(&V) -> Option<V2> + Copy,
  T: VirtualCollection<K, V>,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    let inner_getter = self.inner.access(skip_cache);
    self.inner.iter_key(skip_cache).filter(move |k| {
      let v = inner_getter(k).unwrap();
      (self.checker)(&v).is_some()
    })
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V2> + '_ {
    let inner_getter = self.inner.access(skip_cache);
    move |key| inner_getter(key).and_then(|v| (self.checker)(&v))
  }
}

#[pin_project::pin_project]
pub struct ReactiveKSetFilter<T, KS, K> {
  #[pin]
  inner: T,
  #[pin]
  keys: KS,
  k: PhantomData<K>,
}

#[pin_project::pin_project]
pub struct ReactiveKVUnion<T1, T2, K> {
  #[pin]
  a: T1,
  #[pin]
  b: T2,
  k: PhantomData<K>,
}

impl<T1, T2, K, V1, V2> VirtualCollection<K, (Option<V1>, Option<V2>)>
  for ReactiveKVUnion<T1, T2, K>
where
  K: Copy + std::hash::Hash + Eq,
  T1: VirtualCollection<K, V1>,
  T2: VirtualCollection<K, V2>,
{
  /// we require the T1 T2 has the same key range
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    let mut keys = FastHashSet::<K>::default();
    self.a.iter_key(skip_cache).for_each(|k| {
      keys.insert(k);
    });
    self.b.iter_key(skip_cache).for_each(|k| {
      keys.insert(k);
    });
    keys.into_iter()
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<(Option<V1>, Option<V2>)> + '_ {
    let getter_a = self.a.access(skip_cache);
    let getter_b = self.b.access(skip_cache);

    move |key| Some((getter_a(key), getter_b(key)))
  }
}

impl<T1, T2, K, V1, V2> Stream for ReactiveKVUnion<T1, T2, K>
where
  K: Clone + std::hash::Hash + Eq,
  T1: Stream + Unpin,
  T1::Item: IntoIterator<Item = CollectionDelta<K, V1>>,
  T2: Stream + Unpin,
  T2::Item: IntoIterator<Item = CollectionDelta<K, V2>>,
  T1: VirtualCollection<K, V1>,
  T2: VirtualCollection<K, V2>,
{
  type Item = impl IntoIterator<Item = CollectionDelta<K, (Option<V1>, Option<V2>)>>;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let t1 = self.a.poll_next_unpin(cx);
    let t2 = self.b.poll_next_unpin(cx);

    let a_access = self.a.access(false);
    let b_access = self.b.access(false);

    match (t1, t2) {
      (Poll::Ready(Some(v1)), Poll::Ready(Some(v2))) => {
        let mut intersections: FastHashMap<K, (Option<V1>, Option<V2>)> = FastHashMap::default();
        v1.into_iter().for_each(|d| match d {
          CollectionDelta::Delta(k, v) => {
            intersections.entry(k).or_insert_with(Default::default).0 = Some(v);
          }
          CollectionDelta::Remove(k) => {
            intersections.entry(k).or_insert_with(Default::default).0 = None;
          }
        });
        v2.into_iter().for_each(|d| match d {
          CollectionDelta::Delta(k, v) => {
            intersections.entry(k).or_insert_with(Default::default).1 = Some(v);
          }
          CollectionDelta::Remove(k) => {
            intersections.entry(k).or_insert_with(Default::default).1 = None;
          }
        });

        let output = intersections
          .into_iter()
          .map(|(k, v)| {
            let v_map = match v {
              (Some(v1), Some(v2)) => (Some(v1), Some(v2)),
              (Some(v1), None) => (Some(v1), b_access(&k)),
              (None, Some(v2)) => (a_access(&k), Some(v2)),
              (None, None) => return CollectionDelta::Remove(k),
            };
            CollectionDelta::Delta(k, v_map)
          })
          .collect::<Vec<_>>();

        Poll::Ready(Some(output))
      }
      (Poll::Ready(Some(v1)), Poll::Pending) => Poll::Ready(Some(
        v1.into_iter()
          .map(|v1| {
            let k = v1.key().clone();
            let v1 = v1.value();
            let v2 = b_access(&k);
            match (&v1, &v2) {
              (None, None) => CollectionDelta::Remove(k),
              _ => CollectionDelta::Delta(k, (v1, v2)),
            }
          })
          .collect::<Vec<_>>(),
      )),
      (Poll::Pending, Poll::Ready(Some(v2))) => Poll::Ready(Some(
        v2.into_iter()
          .map(|v2| {
            let k = v2.key().clone();
            let v1 = a_access(&k);
            let v2 = v2.value();
            match (&v1, &v2) {
              (None, None) => CollectionDelta::Remove(k),
              _ => CollectionDelta::Delta(k, (v1, v2)),
            }
          })
          .collect::<Vec<_>>(),
      )),
      (Poll::Pending, Poll::Pending) => Poll::Pending,
      _ => Poll::Ready(None),
    }
  }
}
