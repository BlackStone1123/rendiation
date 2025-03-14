use crate::*;

pub struct ReactiveQueryDebug<T: ReactiveQuery> {
  pub inner: T,
  pub state: RwLock<FastHashMap<T::Key, T::Value>>,
  pub label: &'static str,
  pub log_change: bool,
}

impl<T> ReactiveQuery for ReactiveQueryDebug<T>
where
  T: ReactiveQuery,
{
  type Key = T::Key;
  type Value = T::Value;
  type Changes = T::Changes;
  type View = T::View;
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, v) = self.inner.poll_changes(cx);

    // validation
    let changes = d.materialize();
    let mut state = self.state.write();

    if !changes.is_empty() && self.log_change {
      println!("change details for <{}>:", self.label);
    }
    for (k, change) in changes.iter() {
      if self.log_change {
        println!("{:?}: {:?}", k, change);
      }
      match change {
        ValueChange::Delta(n, p) => {
          if let Some(removed) = state.remove(k) {
            let p = p.as_ref().expect("previous value should exist");
            assert_eq!(&removed, p);
          } else {
            assert!(p.is_none());
          }
          state.insert(k.clone(), n.clone());
        }
        ValueChange::Remove(p) => {
          let removed = state.remove(k).expect("remove none exist value");
          assert_eq!(&removed, p);
        }
      }
    }

    (d, v)
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.inner.request(request)
  }
}

pub struct ReactiveQueryDiff<T> {
  pub inner: T,
}

#[derive(Clone)]
pub struct DiffChangedView<T> {
  inner: T,
}

impl<T, V> Query for DiffChangedView<T>
where
  T: Query<Value = ValueChange<V>>,
  V: CValue,
{
  type Key = T::Key;
  type Value = ValueChange<V>;
  fn iter_key_value(&self) -> impl Iterator<Item = (T::Key, ValueChange<V>)> + '_ {
    self
      .inner
      .iter_key_value()
      .filter(|(_, v)| !v.is_redundant())
  }

  fn access(&self, key: &T::Key) -> Option<ValueChange<V>> {
    let change = self.inner.access(key)?;
    if change.is_redundant() {
      None
    } else {
      Some(change)
    }
  }
}

impl<T> ReactiveQuery for ReactiveQueryDiff<T>
where
  T: ReactiveQuery,
  T::Value: PartialEq,
{
  type Key = T::Key;
  type Value = T::Value;
  type Changes = impl Query<Key = Self::Key, Value = ValueChange<Self::Value>>;
  type View = impl Query<Key = Self::Key, Value = Self::Value>;

  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, v) = self.inner.poll_changes(cx);

    let d = DiffChangedView { inner: d };
    (d, v)
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.inner.request(request);
  }
}

#[pin_project::pin_project]
pub struct ReactiveQueryAsStream<T> {
  #[pin]
  pub inner: T,
}

impl<T> futures::Stream for ReactiveQueryAsStream<T>
where
  T: ReactiveQuery + Unpin,
{
  type Item = Arc<FastHashMap<T::Key, ValueChange<T::Value>>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let r = this.inner.poll_changes(cx).0.materialize();

    if r.is_empty() {
      Poll::Pending
    } else {
      Poll::Ready(Some(r))
    }
  }
}
