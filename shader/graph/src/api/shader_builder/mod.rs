use crate::*;

pub mod binding;
pub use binding::*;
pub mod vertex;
pub use vertex::*;
pub mod fragment;
pub use fragment::*;
pub mod re_export;
pub use re_export::*;
pub mod builtin;
pub use builtin::*;

#[derive(Debug)]
pub enum ShaderGraphBuildError {
  MissingRequiredDependency,
  FragmentOutputSlotNotDeclared,
  FailedDowncastShaderValueFromInput,
}

pub struct ShaderGraphRenderPipelineBuilder {
  // uniforms
  pub bindgroups: ShaderGraphBindGroupBuilder,

  pub(crate) vertex: ShaderGraphVertexBuilder,
  pub(crate) fragment: ShaderGraphFragmentBuilder,
}

impl Default for ShaderGraphRenderPipelineBuilder {
  fn default() -> Self {
    Self {
      bindgroups: Default::default(),
      vertex: ShaderGraphVertexBuilder::new(),
      fragment: ShaderGraphFragmentBuilder::new(),
    }
  }
}

impl std::ops::Deref for ShaderGraphRenderPipelineBuilder {
  type Target = ShaderGraphBindGroupBuilder;

  fn deref(&self) -> &Self::Target {
    &self.bindgroups
  }
}

impl std::ops::DerefMut for ShaderGraphRenderPipelineBuilder {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.bindgroups
  }
}

impl ShaderGraphRenderPipelineBuilder {
  pub fn vertex<T>(
    &mut self,
    logic: impl FnOnce(&mut ShaderGraphVertexBuilder) -> Result<T, ShaderGraphBuildError>,
  ) -> Result<T, ShaderGraphBuildError> {
    set_current_building(true.into());
    let result = logic(&mut self.vertex)?;
    set_current_building(None);
    Ok(result)
  }
  pub fn fragment<T>(
    &mut self,
    logic: impl FnOnce(&mut ShaderGraphFragmentBuilder) -> Result<T, ShaderGraphBuildError>,
  ) -> Result<T, ShaderGraphBuildError> {
    set_current_building(true.into());
    let result = logic(&mut self.fragment)?;
    set_current_building(None);
    Ok(result)
  }

  pub fn build(
    self,
    target: &dyn ShaderGraphCodeGenTarget,
  ) -> Result<ShaderGraphCompileResult, ShaderGraphBuildError> {
    let PipelineShaderGraphPair {
      vertex, fragment, ..
    } = take_build_graph();

    vertex.top_scope_mut().resolve_all_pending();
    let vertex_shader = target.gen_vertex_shader(&self, vertex);

    fragment.top_scope_mut().resolve_all_pending();
    let frag_shader = target.gen_fragment_shader(&self, fragment);

    Ok(ShaderGraphCompileResult {
      vertex_shader,
      frag_shader,
      bindings: self.bindgroups,
      vertex_layouts: self.vertex.vertex_layouts,
      primitive_state: self.vertex.primitive_state,
      color_states: self
        .fragment
        .frag_output
        .iter()
        .cloned()
        .map(|(_, s)| s)
        .collect(),
      depth_stencil: self.fragment.depth_stencil,
      multisample: self.fragment.multisample,
    })
  }
}

/// The reason why we use two function is that the build process
/// require to generate two separate root scope: two entry main function;
pub trait ShaderGraphProvider {
  fn build(
    &self,
    _builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    // default do nothing
    Ok(())
  }
}

impl<'a> ShaderGraphProvider for &'a [&dyn ShaderGraphProvider] {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    for p in *self {
      p.build(builder)?;
    }
    Ok(())
  }
}

pub struct ShaderGraphCompileResult {
  pub vertex_shader: String,
  pub frag_shader: String,
  pub bindings: ShaderGraphBindGroupBuilder,
  pub vertex_layouts: Vec<ShaderGraphVertexBufferLayout>,
  pub primitive_state: PrimitiveState,
  pub color_states: Vec<ColorTargetState>,
  pub depth_stencil: Option<DepthStencilState>,
  pub multisample: MultisampleState,
}

#[derive(Default)]
pub struct SemanticRegistry {
  registered: HashMap<TypeId, NodeMutable<AnyType>>,
}

impl SemanticRegistry {
  pub fn query(&mut self, id: TypeId) -> Result<&NodeMutable<AnyType>, ShaderGraphBuildError> {
    self
      .registered
      .get(&id)
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  }

  pub fn register(&mut self, id: TypeId, node: NodeUntyped) {
    self.registered.entry(id).or_insert_with(|| node.mutable());
  }
}

pub struct SuperUnsafeCell<T> {
  pub data: UnsafeCell<T>,
}

impl<T> SuperUnsafeCell<T> {
  pub fn new(v: T) -> Self {
    Self {
      data: UnsafeCell::new(v),
    }
  }
  #[allow(clippy::mut_from_ref)]
  pub fn get_mut(&self) -> &mut T {
    unsafe { &mut *(self.data.get()) }
  }
  pub fn get(&self) -> &T {
    unsafe { &*(self.data.get()) }
  }
}

unsafe impl<T> Sync for SuperUnsafeCell<T> {}
unsafe impl<T> Send for SuperUnsafeCell<T> {}

#[derive(Default)]
struct PipelineShaderGraphPair {
  vertex: ShaderGraphBuilder,
  fragment: ShaderGraphBuilder,
  current: Option<bool>,
}

static IN_BUILDING_SHADER_GRAPH: once_cell::sync::Lazy<
  SuperUnsafeCell<Option<PipelineShaderGraphPair>>,
> = once_cell::sync::Lazy::new(|| SuperUnsafeCell::new(None));

pub(crate) fn modify_graph<T>(modifier: impl FnOnce(&mut ShaderGraphBuilder) -> T) -> T {
  let graph = IN_BUILDING_SHADER_GRAPH.get_mut().as_mut().unwrap();
  let is_vertex = graph.current.unwrap();
  let graph = if is_vertex {
    &mut graph.vertex
  } else {
    &mut graph.fragment
  };
  modifier(graph)
}

pub(crate) fn set_current_building(current: Option<bool>) {
  let graph = IN_BUILDING_SHADER_GRAPH.get_mut().as_mut().unwrap();
  graph.current = current
}

pub(crate) fn set_build_graph() {
  IN_BUILDING_SHADER_GRAPH
    .get_mut()
    .replace(Default::default());
}

pub(crate) fn take_build_graph() -> PipelineShaderGraphPair {
  IN_BUILDING_SHADER_GRAPH.get_mut().take().unwrap()
}
