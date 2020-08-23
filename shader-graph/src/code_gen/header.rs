use crate::*;

impl ShaderGraphBindGroup {
  pub fn gen_header(&self, graph: &ShaderGraph, index: usize, stage: ShaderStage) -> String {
    self
      .inputs
      .iter()
      .enumerate()
      .filter_map(|(i, h)| {
        if stage != h.1 {
          return None;
        }
        match &h.0 {
          ShaderGraphUniformInputType::NoneUBO(node) => {
            let info = graph.nodes.get_node(*node).data();
            let input = info.unwrap_as_input();
            Some(format!(
              "layout(set = {}, binding = {}) uniform {} {};\n",
              index,
              i,
              graph.type_id_map.get(&info.node_type).unwrap(),
              input.name.as_str()
            ))
          }
          ShaderGraphUniformInputType::UBO((info, _)) => Some(format!(
            "layout(set = {}, binding = {}) {};",
            index, i, info.code_cache
          )),
        }
      })
      .collect::<Vec<_>>()
      .join("\n")
  }
}

impl ShaderGraph {
  pub(super) fn gen_header_vert(&self) -> String {
    let mut result = String::from("#version 450\n");

    // attributes
    result += self
      .attributes
      .iter()
      .map(|a| {
        let info = self.nodes.get_node(a.0).data();
        let input = info.unwrap_as_input();
        format!(
          "layout(location = {}) in {} {};",
          a.1,
          self.type_id_map.get(&info.node_type).unwrap(),
          input.name.as_str()
        )
      })
      .collect::<Vec<String>>()
      .join("\n")
      .as_ref();

    result += self.gen_bindgroups_header(ShaderStage::Vertex).as_str();

    result
  }

  pub(super) fn gen_bindgroups_header(&self, stage: ShaderStage) -> String {
    self
      .bindgroups
      .iter()
      .enumerate()
      .map(|(i, b)| b.gen_header(self, i, stage))
      .collect::<Vec<_>>()
      .join("\n")
  }

  pub(super) fn gen_header_frag(&self) -> String {
    let mut result = String::from("#version 450\n");

    result += self.gen_bindgroups_header(ShaderStage::Fragment).as_str();

    // varyings
    result += self
      .varyings
      .iter()
      .map(|a| {
        let info = self.nodes.get_node(a.0).data();
        // let id = info.unwrap_as_vary();
        format!(
          "layout(location = {}) in {} {};",
          a.1,
          self.type_id_map.get(&info.node_type).unwrap(),
          format!("vary{}", a.1)
        )
      })
      .collect::<Vec<String>>()
      .join("\n")
      .as_ref();

    result
  }
}
