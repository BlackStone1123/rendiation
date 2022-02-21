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

  vertex: ShaderGraphVertexBuilder,
  fragment: ShaderGraphFragmentBuilder,
}

impl Default for ShaderGraphRenderPipelineBuilder {
  fn default() -> Self {
    Self {
      bindgroups: Default::default(),
      vertex: Default::default(),
      fragment: Default::default(),
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
    logic: impl FnOnce(ShaderGraphVertexBuilder) -> Result<T, ShaderGraphBuildError>,
  ) -> Result<T, ShaderGraphBuildError> {
    todo!()
  }
  pub fn fragment<T>(
    &mut self,
    logic: impl FnOnce(ShaderGraphFragmentBuilder) -> Result<T, ShaderGraphBuildError>,
  ) -> Result<T, ShaderGraphBuildError> {
    todo!()
  }

  pub fn build(
    target: &dyn ShaderGraphCodeGenTarget,
  ) -> Result<ShaderGraphCompileResult, ShaderGraphBuildError> {
    todo!()
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

/// entry
pub fn build_shader(
  builder: &ShaderGraphRenderPipelineBuilder,
  target: &dyn ShaderGraphCodeGenTarget,
) -> Result<ShaderGraphCompileResult, ShaderGraphBuildError> {
  let bindgroup_builder = ShaderGraphBindGroupBuilder::default();

  let mut vertex_builder = ShaderGraphVertexBuilder::create(bindgroup_builder);
  builder.build_vertex(&mut vertex_builder)?;
  let mut result = vertex_builder.extract();
  result.top_scope_mut().resolve_all_pending();
  let vertex_shader = target.gen_vertex_shader(&mut vertex_builder, result);

  let vertex_layouts = vertex_builder.vertex_layouts.clone();
  let primitive_state = vertex_builder.primitive_state;

  let mut fragment_builder = ShaderGraphFragmentBuilder::create(vertex_builder);
  builder.build_fragment(&mut fragment_builder)?;
  let mut result = fragment_builder.extract();
  result.top_scope_mut().resolve_all_pending();
  let frag_shader = target.gen_fragment_shader(&mut fragment_builder, result);

  Ok(ShaderGraphCompileResult {
    vertex_shader,
    frag_shader,
    bindings: fragment_builder.bindgroups,
    vertex_layouts,
    primitive_state,
    color_states: fragment_builder
      .frag_output
      .iter()
      .cloned()
      .map(|(_, s)| s)
      .collect(),
    depth_stencil: fragment_builder.depth_stencil,
    multisample: fragment_builder.multisample,
  })
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

pub(crate) fn set_build_graph(g: PipelineShaderGraphPair) {
  IN_BUILDING_SHADER_GRAPH.get_mut().replace(g);
}

pub(crate) fn take_build_graph() -> PipelineShaderGraphPair {
  IN_BUILDING_SHADER_GRAPH.get_mut().take().unwrap()
}
