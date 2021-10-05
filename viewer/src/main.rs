#![feature(capture_disjoint_fields)]
#![feature(array_methods)]
#![feature(min_specialization)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(option_result_unwrap_unchecked)]
#![allow(incomplete_features)]

pub mod scene;
pub use scene::*;

pub mod viewer;
pub use viewer::*;

use interphaser::Application;

fn main() {
  env_logger::builder().init();

  let viewer = Viewer::new();
  let ui = create_ui();

  let viewer = futures::executor::block_on(Application::new(viewer, ui));
  viewer.run();
}
