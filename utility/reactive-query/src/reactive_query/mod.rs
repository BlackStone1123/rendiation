use crate::*;

mod self_contain;
use reactive_stream::DefaultBatchChannel;
pub use self_contain::*;

mod dyn_impl;
pub use dyn_impl::*;

mod operator;
pub use operator::*;

pub enum ReactiveQueryRequest {
  MemoryShrinkToFit,
}

pub trait ReactiveQuery: Sync + Send + 'static {
  type Key: CKey;
  type Value: CValue;
  type Compute: QueryCompute<Key = Self::Key, Value = Self::Value>;

  fn describe(&self, cx: &mut Context) -> Self::Compute;

  fn request(&mut self, request: &mut ReactiveQueryRequest);
}

#[derive(Clone, Default)]
pub struct QueryResolveCtx {
  kept_view: Arc<RwLock<Vec<Box<dyn Any + Send + Sync>>>>,
}

impl QueryResolveCtx {
  pub fn keep_view_alive(&self, view: impl Any + Send + Sync) {
    self.kept_view.write().push(Box::new(view));
  }
}

pub trait QueryCompute: Sync + Send + 'static {
  type Key: CKey;
  type Value: CValue;
  type Changes: Query<Key = Self::Key, Value = ValueChange<Self::Value>> + 'static;
  type View: Query<Key = Self::Key, Value = Self::Value> + 'static;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View);

  fn resolve_with_cx(&mut self) -> ((Self::Changes, Self::View), QueryResolveCtx) {
    let cx = Default::default();
    let r = self.resolve(&cx);
    (r, cx)
  }
}

pub struct AsyncQueryCtx {
  resolve_cx: QueryResolveCtx,
}

pub struct AsyncQuerySpawner;
impl AsyncQuerySpawner {
  pub fn spawn_task<R>(
    &self,
    f: impl FnOnce() -> R + 'static,
  ) -> impl Future<Output = R> + 'static {
    // todo, use some thread pool impl
    async move { f() }
  }
}

// compiler not ready for this.
// pub trait AsyncQueryFutureExt: Future + 'static {
//   fn then_spawn<R: 'static>(
//     self,
//     cx: &mut AsyncQueryCtx,
//     then: impl FnOnce(Self::Output) -> R + 'static,
//   ) -> impl Future<Output = R> + use<>;
// }

// impl<T: Future + 'static> AsyncQueryFutureExt for T {
//   fn then_spawn<R: 'static>(
//     self,
//     cx: &mut AsyncQueryCtx,
//     then: impl FnOnce(Self::Output) -> R + 'static,
//   ) -> impl Future<Output = R> + use<> {
//     let sp = cx.make_spawner();
//     self.then(move |s| sp.spawn_task(move || then(s)))
//   }
// }

impl AsyncQueryCtx {
  pub fn resolve_cx(&self) -> &QueryResolveCtx {
    &self.resolve_cx
  }
  pub fn make_spawner(&self) -> AsyncQuerySpawner {
    AsyncQuerySpawner
  }
  pub fn spawn_task<R>(
    &self,
    f: impl FnOnce() -> R + 'static,
  ) -> impl Future<Output = R> + 'static {
    self.make_spawner().spawn_task(f)
  }

  pub fn then_spawn<T: 'static, R>(
    &self,
    f: impl Future<Output = T> + 'static,
    then: impl FnOnce(T, &QueryResolveCtx) -> R + 'static,
  ) -> impl Future<Output = R> + 'static {
    let sp = self.make_spawner();
    let cx = self.resolve_cx.clone();
    f.then(move |s| sp.spawn_task(move || then(s, &cx)))
  }
}

pub trait AsyncQueryCompute: QueryCompute {
  // this is correct version
  type Task: Future<Output = (Self::Changes, Self::View)> + Send + Sync + 'static;
  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task;
}

impl<K, V, Change, View> QueryCompute for (Change, View)
where
  K: CKey,
  V: CValue,
  Change: Query<Key = K, Value = ValueChange<V>> + 'static,
  View: Query<Key = K, Value = V> + 'static,
{
  type Key = K;
  type Value = V;
  type Changes = Change;
  type View = View;
  fn resolve(&mut self, _cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    (self.0.clone(), self.1.clone())
  }
}
impl<K, V, Change, View> AsyncQueryCompute for (Change, View)
where
  K: CKey,
  V: CValue,
  Change: Query<Key = K, Value = ValueChange<V>> + 'static,
  View: Query<Key = K, Value = V> + 'static,
{
  type Task = impl Future<Output = (Self::Changes, Self::View)> + 'static;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    futures::future::ready(self.resolve(cx.resolve_cx()))
  }
}

impl<K: CKey, V: CValue> ReactiveQuery for EmptyQuery<K, V> {
  type Key = K;
  type Value = V;
  type Compute = (EmptyQuery<K, ValueChange<V>>, EmptyQuery<K, V>);
  fn describe(&self, _: &mut Context) -> Self::Compute {
    (Default::default(), Default::default())
  }
  fn request(&mut self, _: &mut ReactiveQueryRequest) {}
}
