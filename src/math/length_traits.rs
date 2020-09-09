use crate::math::{Length, Point, Size};
pub trait SizeExt<S, U> {
    fn width(&self) -> Length<S, U>;
    fn height(&self) -> Length<S, U>;
    fn set_width(&mut self, width: Length<S, U>);
    fn set_height(&mut self, height: Length<S, U>);
}

impl<S, U> SizeExt<S, U> for Size<S, U>
where
    S: Copy,
{
    fn width(&self) -> Length<S, U> {
        Length::new(self.width)
    }
    fn height(&self) -> Length<S, U> {
        Length::new(self.height)
    }

    fn set_width(&mut self, width: Length<S, U>) {
        self.width = width.get()
    }

    fn set_height(&mut self, height: Length<S, U>) {
        self.height = height.get()
    }
}

pub trait PointExt<S, U> {
    fn x(&self) -> Length<S, U>;
    fn y(&self) -> Length<S, U>;
    fn set_x(&mut self, x: Length<S, U>);
    fn set_y(&mut self, y: Length<S, U>);
}

impl<S, U> PointExt<S, U> for Point<S, U>
where
    S: Copy,
{
    fn x(&self) -> Length<S, U> {
        Length::new(self.x)
    }
    fn y(&self) -> Length<S, U> {
        Length::new(self.y)
    }
    fn set_x(&mut self, x: Length<S, U>) {
        self.x = x.get()
    }
    fn set_y(&mut self, y: Length<S, U>) {
        self.y = y.get()
    }
}
