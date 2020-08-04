use crate::math::{Dimension, Size};

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Surround<S = f32> {
    pub left: S,
    pub top: S,
    pub right: S,
    pub bottom: S,
}

impl<S> Surround<S>
where
    S: Into<Dimension>,
{
    pub fn into_dimensions(self) -> Surround<Dimension> {
        Surround {
            left: self.left.into(),
            top: self.top.into(),
            right: self.right.into(),
            bottom: self.bottom.into(),
        }
    }
}

impl<S> Surround<S>
where
    S: std::ops::Add<Output = S> + Copy,
{
    pub fn minimum_width(&self) -> S {
        self.left + self.right
    }

    pub fn minimum_height(&self) -> S {
        self.top + self.bottom
    }

    pub fn minimum_size(&self) -> Size<S> {
        Size {
            width: self.minimum_width(),
            height: self.minimum_height(),
        }
    }
}

impl<S> Surround<S>
where
    S: Copy,
{
    pub fn uniform(measurement: S) -> Self {
        Self {
            left: measurement,
            top: measurement,
            right: measurement,
            bottom: measurement,
        }
    }
}
