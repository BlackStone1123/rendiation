use std::{hash::Hasher, marker::PhantomData};

use crate::*;

pub struct NodeScope {
  environment: Vec<NodeUntyped>,
}

pub struct IfNode {
  pub condition: Node<bool>,
  // should be same type
  pub true_value: ShaderGraphNodeRawHandleUntyped,
  pub false_value: ShaderGraphNodeRawHandleUntyped,
}

// impl ShaderIterator for Node<ShaderArray<T>> {
//   type Item = T;

//   fn code_gen(&self) -> &'static str {
//     "
//         for(int i = 0; i < 32; i++) {

//         }
//         "
//   }
// }

// let a = 1;
// let c = 0;
// for i in xxx {
//     let b =1;
//     if i> 10 {
//         a+=b
//         continue
//     }
//     c+= i;
// }

// fn test() {
//   let a = node(1);
//   let c = node(0);
//   let b = node(1);
//   xxx.iter().split(
//     until(10).fold(a, |a| a + b),
//     enumerate().fold(a, |a, i| a + i),
//   );
// }

fn test() {
  let a = consts(1).mutable();
  let c = consts(0).mutable();

  for_by(5, |for_ctx, i| {
    let b = 1;
    if_by(i.greater_than(0), || {
      a.set(a.get() + b.into());
      for_ctx.do_continue();
    });
    c.set(c.get() + i);
  });
}

pub struct Mutable<T> {
  phantom: PhantomData<T>,
}

impl<T> Node<T> {
  pub fn mutable(&self) -> Node<Mutable<T>> {
    todo!()
  }
}

impl<T> Node<Mutable<T>> {
  pub fn get(&self) -> Node<T> {
    todo!()
  }

  pub fn set(&self, node: Node<T>) {
    //
  }
}

pub struct ShaderArray<T> {
  phantom: PhantomData<T>,
}

pub trait ShaderIterator {
  type Item;

  fn code_gen(&self) -> &'static str;
}

pub fn consts<T>(v: T) -> Node<T> {
  todo!()
}

pub struct ForCtx {
  //
}

impl ForCtx {
  pub fn do_continue(&self) {
    modify_graph(|builder| {
      let scope = builder.top_scope();
      // todo insert node?
      scope.code_builder.write_ln("continue");
    });
  }

  pub fn do_break(&self) {
    modify_graph(|builder| {
      let scope = builder.top_scope();
      // todo insert node?
      scope.code_builder.write_ln("break");
    });
  }
}

impl ShaderIterator for u32 {
  type Item = u32;

  fn code_gen(&self) -> &'static str {
    todo!()
  }
}

pub fn for_by<T, I>(iterable: I, logic: impl Fn(&ForCtx, Node<T>))
where
  T: ShaderGraphNodeType,
  I: ShaderIterator<Item = T>,
{
  modify_graph(|builder| {
    let scope = builder.top_scope();
    scope.code_builder.write_ln("for ..{");
    scope.code_builder.tab();
    builder.push_scope()
  });

  // input
  let i_node = ShaderGraphNodeData::Named("i".into()).insert_graph();

  let cx = ForCtx {};

  logic(&cx, i_node);

  modify_graph(|builder| {
    let result = builder.pop_scope();
    let result = ShaderGraphNodeData::Scope(result);

    let scope = builder.top_scope();
    result.insert_into_graph::<AnyType>(scope);

    scope.code_builder.un_tab();
    scope.code_builder.write_ln("}");
  });
}

pub fn if_by(condition: impl Into<Node<bool>>, logic: impl Fn()) {
  //
}
