use crate::math::{Length, PointExt, Rect, Size, Unknown, Vector};

#[derive(Copy, Clone, PartialEq, Debug, Default)]
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
        let min = rect.min() + Vector::from_lengths(self.left, self.top);
        let max = rect.max() - Vector::from_lengths(self.right, self.bottom);

        Rect::new(
            min,
            Size::from_lengths(max.x() - min.x(), max.y() - min.y()),
        )
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
