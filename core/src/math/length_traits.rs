use crate::math::{Length, Point, Size, Vector};

/// Extension trait for [`Size`].
pub trait SizeExt<S, U> {
    /// Returns the width as a [`Length`].
    fn width(&self) -> Length<S, U>;
    /// Returns the height as a [`Length`].
    fn height(&self) -> Length<S, U>;
    /// Sets the width from a [`Length`].
    fn set_width(&mut self, width: Length<S, U>);
    /// Sets the height from a [`Length`].
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
        self.width = width.get();
    }

    fn set_height(&mut self, height: Length<S, U>) {
        self.height = height.get();
    }
}

/// Extension trait for [`Point`].
pub trait PointExt<S, U> {
    /// Returns x as a [`Length`].
    fn x(&self) -> Length<S, U>;
    /// Returns y as a [`Length`].
    fn y(&self) -> Length<S, U>;
    /// Sets x from a [`Length`].
    fn set_x(&mut self, x: Length<S, U>);
    /// Sets y from a [`Length`].
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
        self.x = x.get();
    }

    fn set_y(&mut self, y: Length<S, U>) {
        self.y = y.get();
    }
}

impl<S, U> PointExt<S, U> for Vector<S, U>
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
        self.x = x.get();
    }

    fn set_y(&mut self, y: Length<S, U>) {
        self.y = y.get();
    }
}
