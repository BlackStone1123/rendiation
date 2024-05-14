use crate::*;

#[derive(Default)]
pub struct UIGroup {
  children: Vec<Box<dyn View3d>>,
}

impl UIGroup {
  pub fn with_child(mut self, child: impl View3d + 'static) -> Self {
    self.children.push(Box::new(child));
    self
  }
}

impl View3d for UIGroup {
  fn update_view(&mut self, cx: &mut StateCx) {
    for c in &mut self.children {
      c.update_view(cx)
    }
  }
  fn update_state(&mut self, cx: &mut StateCx) {
    for c in &mut self.children {
      c.update_state(cx)
    }
  }
  fn clean_up(&mut self, cx: &mut StateCx) {
    for child in &mut self.children {
      child.clean_up(cx)
    }
  }
}

pub struct UINode {
  node: AllocIdx<SceneNodeEntity>,
  children: UIGroup,
}

impl UINode {
  pub fn new(v: &mut View3dProvider) -> Self {
    todo!()
  }
  pub fn node(&self) -> AllocIdx<SceneNodeEntity> {
    self.node
  }
  pub fn with_child<V: View3d + 'static>(
    mut self,
    v: &mut View3dProvider,
    child: impl FnOnce(AllocIdx<SceneNodeEntity>, &mut View3dProvider) -> V,
  ) -> Self {
    self.children = self.children.with_child(child(self.node, v));
    self
  }
}

impl View3d for UINode {
  fn update_view(&mut self, cx: &mut StateCx) {
    self.children.update_view(cx)
  }

  fn update_state(&mut self, cx: &mut StateCx) {
    self.children.update_state(cx)
  }

  fn clean_up(&mut self, cx: &mut StateCx) {
    todo!();
    self.children.clean_up(cx)
  }
}
