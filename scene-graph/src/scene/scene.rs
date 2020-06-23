use super::{background::Background, node::SceneNode, resource::ResourceManager};
use crate::{
  GeometryHandle, RenderData, RenderObject, RenderObjectHandle, SceneGraphBackend, SceneNodeHandle,
  ShadingHandle, UniformHandle,
};
use arena::{Arena, Handle};
use rendiation_render_entity::{Camera, PerspectiveCamera};

pub struct ResourceUpdateCtx<T: SceneGraphBackend> {
  changed_uniforms: Vec<UniformHandle<T>>,
}

impl<T: SceneGraphBackend> ResourceUpdateCtx<T> {
  pub fn new() -> Self {
    Self {
      changed_uniforms: Vec::new(),
    }
  }
  pub fn notify_uniform_update(&mut self, index: UniformHandle<T>) {
    self.changed_uniforms.push(index)
  }
}

pub type CameraHandle = Handle<Box<dyn Camera>>;

pub struct CameraData {
  active_camera_index: CameraHandle,
  cameras: Arena<Box<dyn Camera>>,
}

impl CameraData {
  pub fn set_new_active_camera(&mut self, camera: impl Camera + 'static) -> CameraHandle {
    let boxed = Box::new(camera);
    let index = self.cameras.insert(boxed);
    self.active_camera_index = index;
    index
  }

  pub fn get_active_camera_mut_any(&mut self) -> &mut Box<dyn Camera> {
    self.cameras.get_mut(self.active_camera_index).unwrap()
  }

  pub fn get_active_camera_mut<U: 'static>(&mut self) -> &mut U {
    self
      .cameras
      .get_mut(self.active_camera_index)
      .unwrap()
      .as_any_mut()
      .downcast_mut::<U>()
      .unwrap()
  }

  pub fn get_active_camera<U: 'static>(&mut self) -> &U {
    self
      .cameras
      .get(self.active_camera_index)
      .unwrap()
      .as_any()
      .downcast_ref::<U>()
      .unwrap()
  }
}

pub struct Scene<T: SceneGraphBackend> {
  pub background: Option<Box<dyn Background<T>>>,
  pub cameras: CameraData,
  pub render_objects: Arena<RenderObject<T>>,

  root: SceneNodeHandle<T>,
  pub(crate) nodes: Arena<SceneNode<T>>,

  pub resources: ResourceManager<T>,
  pub resource_update_ctx: ResourceUpdateCtx<T>,
}

impl<T: SceneGraphBackend> Scene<T> {
  pub fn new() -> Self {
    let camera_default = Box::new(PerspectiveCamera::new());

    let mut cameras: Arena<Box<dyn Camera>> = Arena::new();
    let active_camera_index = cameras.insert(camera_default);

    let mut nodes = Arena::new();

    let root = SceneNode::new();
    let index = nodes.insert(root);
    nodes.get_mut(index).unwrap().set_self_id(index);

    Self {
      background: None,
      cameras: CameraData {
        active_camera_index,
        cameras,
      },
      render_objects: Arena::new(),
      root: index,
      nodes,
      resources: ResourceManager::new(),
      resource_update_ctx: ResourceUpdateCtx::new(),
    }
  }

  pub fn get_parent_child_pair(
    &mut self,
    parent_id: SceneNodeHandle<T>,
    child_id: SceneNodeHandle<T>,
  ) -> (&mut SceneNode<T>, &mut SceneNode<T>) {
    let (parent, child) = self.nodes.get2_mut(parent_id, child_id);
    (parent.unwrap(), child.unwrap())
  }

  pub fn node_add_child_by_id(
    &mut self,
    parent_id: SceneNodeHandle<T>,
    child_id: SceneNodeHandle<T>,
  ) {
    let (parent, child) = self.get_parent_child_pair(parent_id, child_id);
    parent.add(child);
  }

  pub fn node_remove_child_by_id(
    &mut self,
    parent_id: SceneNodeHandle<T>,
    child_id: SceneNodeHandle<T>,
  ) {
    let (parent, child) = self.get_parent_child_pair(parent_id, child_id);
    parent.remove(child);
  }

  pub fn add_to_scene_root(&mut self, child_id: SceneNodeHandle<T>) {
    self.node_add_child_by_id(self.root, child_id);
  }

  pub fn get_root_node_mut(&mut self) -> &mut SceneNode<T> {
    self.get_node_mut(self.root)
  }

  pub fn get_node(&self, index: SceneNodeHandle<T>) -> &SceneNode<T> {
    self.nodes.get(index).unwrap()
  }

  pub fn get_root(&self) -> &SceneNode<T> {
    self.nodes.get(self.root).unwrap()
  }

  pub fn get_node_mut(&mut self, index: SceneNodeHandle<T>) -> &mut SceneNode<T> {
    self.nodes.get_mut(index).unwrap()
  }

  pub fn create_new_node(&mut self) -> &mut SceneNode<T> {
    let new_node = SceneNode::new();
    let index = self.nodes.insert(new_node);
    let new_node = self.nodes.get_mut(index).unwrap().set_self_id(index);
    new_node
  }

  pub fn get_node_render_data(&self, id: SceneNodeHandle<T>) -> &RenderData {
    &self.nodes.get(id).unwrap().render_data
  }

  pub fn free_node(&mut self, index: SceneNodeHandle<T>) {
    self.nodes.remove(index);
  }

  pub fn create_render_object(
    &mut self,
    geometry_index: GeometryHandle<T>,
    shading_index: ShadingHandle<T>,
  ) -> RenderObjectHandle<T> {
    let obj = RenderObject {
      render_order: 0,
      shading_index,
      geometry_index,
    };
    self.render_objects.insert(obj)
  }

  pub fn delete_render_object(&mut self, index: RenderObjectHandle<T>) {
    self.render_objects.remove(index);
  }

  pub fn traverse(
    &mut self,
    start_index: SceneNodeHandle<T>,
    visit_stack: &mut Vec<SceneNodeHandle<T>>,
    mut visitor: impl FnMut(&mut SceneNode<T>, Option<&mut SceneNode<T>>),
  ) {
    visit_stack.clear();
    visit_stack.push(start_index);

    while let Some(index) = visit_stack.pop() {
      if let Some(parent_index) = self.get_node(index).parent {
        let (parent, this) = self.get_parent_child_pair(parent_index, index);
        visitor(this, Some(parent));
        visit_stack.extend(this.children.iter().cloned())
      } else {
        let this = self.get_node_mut(index);
        visitor(this, None);
        visit_stack.extend(this.children.iter().cloned())
      }
    }
  }
}
