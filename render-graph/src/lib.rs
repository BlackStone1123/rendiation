pub use arena_graph::*;
pub use rendiation_math::*;
pub use rendiation_render_entity::*;
use std::{
  cell::{Cell, RefCell},
  collections::HashSet,
};

mod backend;
mod executor;
mod nodes;
mod target_pool;
pub use backend::*;
pub use executor::*;
pub use nodes::*;
pub use target_pool::*;

pub type RenderGraphNodeHandle = ArenaGraphNodeHandle<RenderGraphNode>;

pub struct RenderGraph {
  graph: RefCell<ArenaGraph<RenderGraphNode>>,
  root_handle: Cell<Option<RenderGraphNodeHandle>>,
  pass_queue: RefCell<Option<Vec<PassExecuteInfo>>>,
}

impl RenderGraph {
  pub fn new() -> Self {
    Self {
      graph: RefCell::new(ArenaGraph::new()),
      root_handle: Cell::new(None),
      pass_queue: RefCell::new(None),
    }
  }

  pub fn pass(&self, name: &str) -> PassNodeBuilder {
    let handle = self
      .graph
      .borrow_mut()
      .new_node(RenderGraphNode::Pass(PassNodeData {
        name: name.to_owned(),
        viewport: Viewport::new((1, 1)),
        input_targets_map: HashSet::new(),
        render: None,
      }));
    PassNodeBuilder {
      builder: NodeBuilder {
        handle,
        graph: self,
      },
    }
  }

  pub fn screen(&self) -> TargetNodeBuilder {
    let handle = self
      .graph
      .borrow_mut()
      .new_node(RenderGraphNode::Target(TargetNodeData::screen()));
    self.root_handle.set(Some(handle));

    TargetNodeBuilder {
      builder: NodeBuilder {
        handle,
        graph: self,
      },
    }
  }

  pub fn target(&self, name: &str) -> TargetNodeBuilder {
    let handle = self
      .graph
      .borrow_mut()
      .new_node(RenderGraphNode::Target(TargetNodeData::target(
        name.to_owned(),
      )));

    TargetNodeBuilder {
      builder: NodeBuilder {
        handle,
        graph: self,
      },
    }
  }
}

fn build_pass_queue(graph: &RenderGraph) -> Vec<PassExecuteInfo> {
  let root = graph.root_handle.get().unwrap();
  let graph = graph.graph.borrow_mut();
  let node_list: Vec<RenderGraphNodeHandle> = graph
    .topological_order_list(root)
    .into_iter()
    .filter(|n| graph.get_node_data_by_node(*n).is_pass())
    .collect();

  let mut exe_info_list: Vec<PassExecuteInfo> = node_list
    .iter()
    .map(|&n| PassExecuteInfo {
      pass_node_handle: n,
      target_drop_list: Vec::new(),
    })
    .collect();
  node_list.iter().enumerate().for_each(|(index, &n)| {
    let node = graph.get_node(n);
    let output_node = *node.to().iter().next().unwrap();
    let output_node_data = graph.get_node_data_by_node(output_node);
    if output_node_data.unwrap_target_data().is_screen() {
      return;
    }
    let mut last_used_index = node_list.len();
    node_list
      .iter()
      .enumerate()
      .skip(index)
      .rev()
      .take_while(|&(rev_index, n)| {
        let check_node = graph.get_node(*n);
        let result = check_node.from().contains(&output_node);
        if result {
          last_used_index = rev_index;
        }
        result
      })
      .for_each(|_| {});
    let list = &mut exe_info_list[last_used_index].target_drop_list;
    if list.iter().position(|&x| x == output_node).is_none() {
      list.push(output_node)
    }
  });
  exe_info_list
}

pub fn build_test_graph() {
  let graph = RenderGraph::new();
  let normal_pass = graph.pass("normal").viewport();
  let normal_target = graph.target("normal").from_pass(&normal_pass);
  let copy_screen = graph.pass("copy_screen").viewport().depend(&normal_target);
  graph.screen().from_pass(&copy_screen);
}
