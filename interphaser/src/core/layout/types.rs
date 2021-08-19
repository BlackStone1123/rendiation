use crate::LayoutResult;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConstraint {
  pub min: LayoutSize,
  pub max: LayoutSize,
}

impl Default for LayoutConstraint {
  fn default() -> Self {
    Self::UNBOUNDED
  }
}

impl LayoutConstraint {
  /// An unbounded box constraints object.
  ///
  /// Can be satisfied by any nonnegative size.
  pub const UNBOUNDED: Self = Self {
    min: LayoutSize::ZERO,
    max: LayoutSize::new(f32::INFINITY, f32::INFINITY),
  };

  /// Create a new box constraints object.
  ///
  /// Create constraints based on minimum and maximum size.
  ///
  /// The given sizes are also [rounded away from zero],
  /// so that the layout is aligned to integers.
  ///
  /// [rounded away from zero]: struct.Size.html#method.expand
  pub fn new(min: LayoutSize, max: LayoutSize) -> Self {
    Self { min, max }
  }
  /// Create a "tight" box constraints object.
  ///
  /// A "tight" constraint can only be satisfied by a single size.
  ///
  /// The given size is also [rounded away from zero],
  /// so that the layout is aligned to integers.
  ///
  /// [rounded away from zero]: struct.Size.html#method.expand
  pub fn tight(size: LayoutSize) -> Self {
    Self {
      min: size,
      max: size,
    }
  }

  /// Create a "loose" version of the constraints.
  ///
  /// Make a version with zero minimum size, but the same maximum size.
  pub fn loosen(&self) -> Self {
    Self {
      min: LayoutSize::ZERO,
      max: self.max,
    }
  }

  /// Clamp a given size so that it fits within the constraints.
  ///
  /// The given size is also [rounded away from zero],
  /// so that the layout is aligned to integers.
  ///
  /// [rounded away from zero]: struct.Size.html#method.expand
  pub fn constrain(&self, size: impl Into<LayoutSize>) -> LayoutSize {
    size.into().clamp(self.min, self.max)
  }

  pub fn from_max(size: LayoutSize) -> Self {
    Self {
      min: LayoutSize::ZERO,
      max: size,
    }
  }
  pub fn max(&self) -> LayoutSize {
    self.max
  }
  pub fn min(&self) -> LayoutSize {
    self.min
  }
  pub fn clamp(&self, size: LayoutSize) -> LayoutSize {
    LayoutSize {
      width: size.width.clamp(self.min.width, self.max.width),
      height: size.height.clamp(self.min.height, self.max.height),
    }
  }

  /// Shrink min and max constraints by size
  ///
  /// The given size is also [rounded away from zero],
  /// so that the layout is aligned to integers.
  ///
  /// [rounded away from zero]: struct.Size.html#method.expand
  pub fn shrink(&self, diff: impl Into<LayoutSize>) -> Self {
    let diff = diff.into();
    let min = LayoutSize::new(
      (self.min().width - diff.width).max(0.),
      (self.min().height - diff.height).max(0.),
    );
    let max = LayoutSize::new(
      (self.max().width - diff.width).max(0.),
      (self.max().height - diff.height).max(0.),
    );

    Self::new(min, max)
  }

  /// Test whether these constraints contain the given `Size`.
  pub fn contains(&self, size: impl Into<LayoutSize>) -> bool {
    let size = size.into();
    (self.min.width <= size.width && size.width <= self.max.width)
      && (self.min.height <= size.height && size.height <= self.max.height)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LayoutSize {
  pub width: f32,
  pub height: f32,
}

impl LayoutSize {
  pub const ZERO: Self = Self {
    width: 0.,
    height: 0.,
  };
  pub const fn new(width: f32, height: f32) -> Self {
    Self { width, height }
  }

  pub fn with_default_baseline(self) -> LayoutResult {
    LayoutResult {
      size: self,
      baseline_offset: 0.,
    }
  }

  pub fn clamp(self, min: Self, max: Self) -> Self {
    let width = self.width.max(min.width).min(max.width);
    let height = self.height.max(min.height).min(max.height);
    Self { width, height }
  }
}

impl<T: Into<f32>> From<(T, T)> for LayoutSize {
  fn from(value: (T, T)) -> Self {
    Self {
      width: value.0.into(),
      height: value.1.into(),
    }
  }
}

impl<T: From<f32>> From<LayoutSize> for (T, T) {
  fn from(value: LayoutSize) -> Self {
    (value.width.into(), value.height.into())
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UIPosition {
  pub x: f32,
  pub y: f32,
}

impl From<(f32, f32)> for UIPosition {
  fn from(v: (f32, f32)) -> Self {
    Self { x: v.0, y: v.1 }
  }
}
