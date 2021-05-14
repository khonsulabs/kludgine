#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::math::{Box2D, Length, Rect, Scale, Size, Unknown, Vector};

#[derive(Copy, Clone, PartialEq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Surround<S = f32, Unit = Unknown> {
    pub left: Length<S, Unit>,
    pub top: Length<S, Unit>,
    pub right: Length<S, Unit>,
    pub bottom: Length<S, Unit>,
}

impl<S, Unit> Surround<S, Unit>
where
    S: std::ops::Add<Output = S> + Copy,
{
    pub fn minimum_width(&self) -> Length<S, Unit> {
        self.left + self.right
    }

    pub fn minimum_height(&self) -> Length<S, Unit> {
        self.top + self.bottom
    }

    pub fn minimum_size(&self) -> Size<S, Unit> {
        Size::from_lengths(self.minimum_width(), self.minimum_height())
    }
}

impl<S, Unit> Surround<S, Unit>
where
    S: std::ops::Add<Output = S> + std::ops::Sub<Output = S> + Copy,
{
    pub fn inset_rect(&self, rect: &Rect<S, Unit>) -> Rect<S, Unit> {
        let rect = rect.to_box2d();
        let min = rect.min + Vector::from_lengths(self.left, self.top);
        let max = rect.max - Vector::from_lengths(self.right, self.bottom);

        Box2D::new(min, max).to_rect()
    }

    pub fn inset_constraints(&self, constraints: &Size<Option<S>, Unit>) -> Size<Option<S>, Unit> {
        let width = constraints
            .width
            .map(|width| width - self.minimum_width().get());
        let height = constraints
            .height
            .map(|height| height - self.minimum_width().get());

        Size::new(width, height)
    }
}

impl<T, Unit> std::ops::Sub for Surround<T, Unit>
where
    T: std::ops::Sub<Output = T>,
{
    type Output = Surround<T, Unit>;

    fn sub(self, rhs: Self) -> Self::Output {
        Surround {
            left: self.left - rhs.left,
            right: self.right - rhs.right,
            top: self.top - rhs.top,
            bottom: self.bottom - rhs.bottom,
        }
    }
}

impl<Src, Dst> std::ops::Mul<Scale<f32, Src, Dst>> for Surround<f32, Src> {
    type Output = Surround<f32, Dst>;

    fn mul(self, rhs: Scale<f32, Src, Dst>) -> Self::Output {
        Surround {
            left: self.left * rhs,
            right: self.right * rhs,
            top: self.top * rhs,
            bottom: self.bottom * rhs,
        }
    }
}

impl<Src, Dst> std::ops::Div<Scale<f32, Src, Dst>> for Surround<f32, Dst> {
    type Output = Surround<f32, Src>;

    fn div(self, rhs: Scale<f32, Src, Dst>) -> Self::Output {
        Surround {
            left: self.left / rhs,
            right: self.right / rhs,
            top: self.top / rhs,
            bottom: self.bottom / rhs,
        }
    }
}

impl<S, Unit> Surround<S, Unit>
where
    S: Copy,
{
    pub fn uniform(measurement: Length<S, Unit>) -> Self {
        Self {
            left: measurement,
            top: measurement,
            right: measurement,
            bottom: measurement,
        }
    }
}
