pub mod dimension3;
pub use dimension3::*;
pub mod dimension2;
pub use dimension2::*;

pub mod aabb;
pub mod line_segment;
pub mod md_circle;
pub mod md_line;
pub mod point;
pub mod ray;
pub mod triangle;

pub use aabb::*;
pub use line_segment::*;
pub use md_circle::*;
pub use md_line::*;
pub use point::*;
pub use ray::*;
pub use triangle::*;

pub mod transformation;
pub use transformation::*;

pub trait IntersectAble<Target, Result, Parameter = ()> {
  fn intersect(&self, other: &Target, param: &Parameter) -> Result;
}

#[macro_export]
macro_rules! intersect_reverse {
  ($self_item: ty, $result:ty, $param:ty, $target:ty) => {
    impl IntersectAble<$target, $result, $param> for $self_item {
      fn intersect(&self, other: &$target, p: &$param) -> $result {
        IntersectAble::<$self_item, $result, $param>::intersect(other, self, p)
      }
    }
  };
}
