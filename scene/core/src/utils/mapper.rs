use std::{
  collections::{HashMap, HashSet},
  marker::PhantomData,
  sync::{Arc, RwLock},
};

use incremental::Incremental;

use super::identity::Identity;

pub struct IdentityMapper<T, U: Incremental> {
  extra_change_source: Option<Box<dyn Fn(&Identity<U>, &Arc<RwLock<ChangeRecorder>>, usize)>>,
  data: HashMap<usize, (T, bool)>,
  changes: Arc<RwLock<ChangeRecorder>>,
  phantom: PhantomData<U>,
}

#[derive(Default)]
pub struct ChangeRecorder {
  pub to_remove: Vec<usize>,
  pub changed: HashSet<usize>,
}

impl<T, U: Incremental> Default for IdentityMapper<T, U> {
  fn default() -> Self {
    Self {
      extra_change_source: None,
      data: Default::default(),
      changes: Default::default(),
      phantom: Default::default(),
    }
  }
}

pub enum ResourceLogic<'a, 'b, T, U> {
  Create(&'a U),
  Update(&'b mut T, &'a U),
}
pub enum ResourceLogicResult<'a, T> {
  Create(T),
  Update(&'a mut T),
}

impl<'a, T> ResourceLogicResult<'a, T> {
  pub fn unwrap_new(self) -> T {
    match self {
      ResourceLogicResult::Create(v) => v,
      ResourceLogicResult::Update(_) => panic!(),
    }
  }

  pub fn unwrap_update(self) -> &'a mut T {
    match self {
      ResourceLogicResult::Create(_) => panic!(),
      ResourceLogicResult::Update(v) => v,
    }
  }
}

impl<T: 'static, U: Incremental> IdentityMapper<T, U> {
  pub fn with_extra_source(
    mut self,
    extra: impl Fn(&Identity<U>, &Arc<RwLock<ChangeRecorder>>, usize) + 'static,
  ) -> Self {
    self.extra_change_source = Some(Box::new(extra));
    self
  }

  pub fn check_clean_up(&mut self) {
    let mut changes = self.changes.write().unwrap();
    changes.to_remove.drain(..).for_each(|id| {
      self.data.remove(&id);
    });
    changes.changed.drain().for_each(|id| {
      self.data.get_mut(&id).unwrap().1 = true;
    })
  }

  /// this to bypass the borrow limits of get_update_or_insert_with
  pub fn get_update_or_insert_with_logic<'a, 'b>(
    &'b mut self,
    source: &'a Identity<U>,
    mut logic: impl FnMut(ResourceLogic<'a, 'b, T, U>) -> ResourceLogicResult<'b, T>,
  ) -> &'b mut T {
    self.check_clean_up();

    let mut new_created = false;
    let id = source.id;

    let (resource, is_dirty) = self.data.entry(id).or_insert_with(|| {
      let item = logic(ResourceLogic::Create(&source.inner)).unwrap_new();
      new_created = true;

      let weak_changed = Arc::downgrade(&self.changes);
      source.delta_stream.on(move |_| {
        if let Some(change) = weak_changed.upgrade() {
          change.write().unwrap().changed.insert(id);
          false
        } else {
          true
        }
      });

      if let Some(extra) = &self.extra_change_source {
        extra(source, &self.changes, id);
      }

      let weak_to_remove = Arc::downgrade(&self.changes);
      source.drop_stream.on(move |_| {
        if let Some(to_remove) = weak_to_remove.upgrade() {
          to_remove.write().unwrap().to_remove.push(id);
          false
        } else {
          true
        }
      });

      (item, false)
    });

    if new_created || *is_dirty {
      *is_dirty = false;
      logic(ResourceLogic::Update(resource, source)).unwrap_update()
    } else {
      resource
    }
  }

  pub fn get_update_or_insert_with(
    &mut self,
    source: &Identity<U>,
    mut creator: impl FnMut(&U) -> T,
    mut updater: impl FnMut(&mut T, &U),
  ) -> &mut T {
    self.get_update_or_insert_with_logic(source, |logic| match logic {
      ResourceLogic::Create(source) => ResourceLogicResult::Create(creator(source)),
      ResourceLogic::Update(mapped, source) => {
        updater(mapped, source);
        ResourceLogicResult::Update(mapped)
      }
    })
  }

  pub fn get_unwrap<X: Incremental>(&self, source: &Identity<X>) -> &T {
    &self.data.get(&source.id).unwrap().0
  }
}
