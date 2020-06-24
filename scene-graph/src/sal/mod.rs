// sal for Shading abstraction layer

// pub fn glsl(){

// }

use arena::{Arena, Handle};

fn test() {
  let tone_mapping = glsl(
    "
    vec3 Uncharted2ToneMapping(
        vec3 intensity, 
        float toneMappingExposure,
        float toneMappingWhitePoint
        ) {
          intensity *= toneMappingExposure;
          return Uncharted2Helper(intensity) / Uncharted2Helper(vec3(toneMappingWhitePoint));
      }
    ",
  );
}

pub struct ShaderFunctionLib {
  functions: Arena<ShaderFunction>,
}

pub struct ShaderFunction {
  source: &'static str,
  depend_function: Vec<Handle<Self>>,
}

impl ShaderFunction {
  pub fn with_depend_fn(handle: Handle<Self>) {}
}

pub fn glsl(source: &'static str) -> ShaderFunction {
  ShaderFunction {
    source,
    depend_function: Vec::new(),
  }
}

pub struct ShaderFunctionNode{
    function: Handle<ShaderFunction>,

}

pub struct ShaderGraph{
    nodes: Arena<ShaderFunctionNode>
}

fn test2() {
    // let MyMaterial = graph!("MyMaterial", [
    //     Camera, 
    //     Phong,
    // ]);
    let base = BaseShading::new();
    let mvp_base = PositionAttributeInput::new(base);
    mvp_base/
}


struct BaseShading{
    graph: ShaderGraph,
}

impl BaseShading{
    pub fn new() -> Self {
        todo!()
    }

    // pub fn mvp(&mut self) -> Self {todo!()} 
}

pub struct Node{}

pub struct PositionAttributeInput<T>{
    before: T,
    attibute_input_node: Node,
}

impl<T> PositionAttributeInput<T> {
    pub fn new(before: T) -> Self{
        todo!()
    }
}

pub struct MVPTransform{
    
}


pub trait ShaderGraphDecorator{
    fn decorate(graph: &mut ShaderGraph);
}