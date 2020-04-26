pub type Rect<S = f32> = rgx::rect::Rect<S>;

#[derive(Copy, Clone, Default, Debug)]
pub struct Point<S = f32> {
    pub x: S,
    pub y: S,
}

impl<S> Point<S> {
    pub fn new(x: S, y: S) -> Self {
        Self { x, y }
    }
}

impl<S> std::ops::Div<S> for Point<S>
where
    S: std::ops::Div<Output = S> + Copy,
{
    type Output = Self;

    fn div(self, s: S) -> Self {
        Self {
            x: self.x / s,
            y: self.y / s,
        }
    }
}

impl<S> std::ops::Mul<S> for Point<S>
where
    S: std::ops::Mul<Output = S> + Copy,
{
    type Output = Self;

    fn mul(self, s: S) -> Self {
        Self {
            x: self.x * s,
            y: self.y * s,
        }
    }
}

impl<S> Into<rgx::math::Point2<S>> for Point<S> {
    fn into(self) -> rgx::math::Point2<S> {
        rgx::math::Point2::new(self.x, self.y)
    }
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Size<S = f32> {
    pub width: S,
    pub height: S,
}

impl<S> Size<S> {
    pub fn new(width: S, height: S) -> Self {
        Size { width, height }
    }
}

impl Into<winit::dpi::Size> for Size {
    fn into(self) -> winit::dpi::Size {
        winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(
            self.width as u32,
            self.height as u32,
        ))
    }
}

pub trait KludgineRect {
    fn new(origin: Point, size: Size) -> Self;
    fn union(&self, other: &Self) -> Self;
}

impl KludgineRect for Rect {
    fn new(origin: Point, size: Size) -> Self {
        Rect::sized(origin.x, origin.y, size.width, size.height)
    }

    fn union(&self, other: &Self) -> Self {
        let min_x = if self.x1 < other.x1 {
            self.x1
        } else {
            other.x1
        };
        let min_y = if self.y1 < other.y1 {
            self.y1
        } else {
            other.y1
        };
        let max_x = if self.x2 > other.x2 {
            self.x2
        } else {
            other.x2
        };
        let max_y = if self.y2 > other.y2 {
            self.y2
        } else {
            other.y2
        };
        Self::new(min_x, min_y, max_x, max_y)
    }
}

pub trait Zeroable {
    fn zero() -> Self;
}

impl<S> Zeroable for Point<S>
where
    S: Default,
{
    fn zero() -> Self {
        Self::new(S::default(), S::default())
    }
}
