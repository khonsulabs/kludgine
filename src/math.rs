#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Rect<S = f32> {
    pub origin: Point<S>,
    pub size: Size<S>,
}

impl<S> Rect<S>
where
    S: std::ops::Sub<Output = S> + std::ops::Add<Output = S> + Copy + PartialOrd,
{
    pub fn sized(origin: Point<S>, size: Size<S>) -> Self {
        Self { origin, size }
    }

    pub fn new(min: Point<S>, max: Point<S>) -> Self {
        Self {
            origin: min,
            size: Size::new(max.x - min.x, max.y - min.y),
        }
    }

    pub fn x1(&self) -> S {
        self.origin.x
    }

    pub fn y1(&self) -> S {
        self.origin.y
    }

    pub fn x2(&self) -> S {
        self.origin.x + self.size.width
    }

    pub fn y2(&self) -> S {
        self.origin.y + self.size.height
    }

    pub fn union(&self, other: &Self) -> Self {
        let min_x = if self.x1() < other.x1() {
            self.x1()
        } else {
            other.x1()
        };
        let min_y = if self.y1() < other.y1() {
            self.y1()
        } else {
            other.y1()
        };
        let max_x = if self.x2() > other.x2() {
            self.x2()
        } else {
            other.x2()
        };
        let max_y = if self.y2() > other.y2() {
            self.y2()
        } else {
            other.y2()
        };
        Self::new(Point::new(min_x, min_y), Point::new(max_x, max_y))
    }

    pub fn inset(&self, surround: Surround<S>) -> Self {
        Self::new(
            Point::new(self.x1() + surround.left, self.y1() + surround.top),
            Point::new(self.x2() - surround.right, self.y2() - surround.bottom),
        )
    }

    pub fn contains(&self, point: Point<S>) -> bool {
        self.origin.x <= point.x
            && self.x2() >= point.x
            && self.origin.y <= point.y
            && self.y2() >= point.y
    }
}

impl<S> Into<rgx::rect::Rect<S>> for Rect<S>
where
    S: std::ops::Sub<Output = S> + std::ops::Add<Output = S> + Copy + PartialOrd,
{
    fn into(self) -> rgx::rect::Rect<S> {
        rgx::rect::Rect::new(self.x1(), self.y1(), self.x2(), self.y2())
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq)]
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

#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct Size<S = f32> {
    pub width: S,
    pub height: S,
}

impl<S> Size<S> {
    pub const fn new(width: S, height: S) -> Self {
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

impl<S> std::ops::Div<S> for Size<S>
where
    S: std::ops::Div<Output = S> + Copy,
{
    type Output = Self;

    fn div(self, s: S) -> Self {
        Self {
            width: self.width / s,
            height: self.height / s,
        }
    }
}

impl<S> std::ops::Mul<S> for Size<S>
where
    S: std::ops::Mul<Output = S> + Copy,
{
    type Output = Self;

    fn mul(self, s: S) -> Self {
        Self {
            width: self.width * s,
            height: self.height * s,
        }
    }
}

impl<S> std::ops::Div<Size<S>> for Size<S>
where
    S: std::ops::Div<Output = S> + Copy,
{
    type Output = Self;

    fn div(self, s: Size<S>) -> Self {
        Self {
            width: self.width / s.width,
            height: self.height / s.height,
        }
    }
}

impl<S> std::ops::Sub<Size<S>> for Size<S>
where
    S: std::ops::Sub<Output = S> + Copy,
{
    type Output = Self;

    fn sub(self, s: Size<S>) -> Self {
        Self {
            width: self.width - s.width,
            height: self.height - s.height,
        }
    }
}

impl<S> std::ops::Add<Size<S>> for Size<S>
where
    S: std::ops::Add<Output = S> + Copy,
{
    type Output = Self;

    fn add(self, s: Size<S>) -> Self {
        Self {
            width: self.width + s.width,
            height: self.height + s.height,
        }
    }
}

impl<S> std::ops::Add<Size<S>> for Point<S>
where
    S: std::ops::Add<Output = S> + Copy,
{
    type Output = Self;

    fn add(self, s: Size<S>) -> Self {
        Self {
            x: self.x + s.width,
            y: self.y + s.height,
        }
    }
}

impl<S> std::ops::Sub<Size<S>> for Point<S>
where
    S: std::ops::Sub<Output = S> + Copy,
{
    type Output = Self;

    fn sub(self, s: Size<S>) -> Self {
        Self {
            x: self.x - s.width,
            y: self.y - s.height,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Surround<S> {
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
    S: Into<Dimension> + Copy,
{
    pub fn minimum_width(&self) -> f32 {
        self.left.into().points().unwrap_or(0.0) + self.right.into().points().unwrap_or(0.0)
    }

    pub fn minimum_height(&self) -> f32 {
        self.top.into().points().unwrap_or(0.0) + self.bottom.into().points().unwrap_or(0.0)
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

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Dimension {
    Auto,
    /// Scale-corrected to the users preference of DPI
    Points(f32),
}

impl Dimension {
    pub fn is_auto(&self) -> bool {
        self == &Dimension::Auto
    }

    pub fn points(&self) -> Option<f32> {
        if let Dimension::Points(points) = &self {
            Some(*points)
        } else {
            None
        }
    }
}

impl Default for Dimension {
    fn default() -> Self {
        Dimension::Auto
    }
}

impl From<f32> for Dimension {
    fn from(value: f32) -> Self {
        Dimension::Points(value)
    }
}

pub fn max_f(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

pub fn min_f(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(Point::new(1i32, 10), Point::new(3, 12));
        // lower x, lower y
        assert!(!rect.contains(Point::new(0, 9)));
        // lower x, equal y
        assert!(!rect.contains(Point::new(0, 10)));
        // equal x, lower y
        assert!(!rect.contains(Point::new(1, 9)));
        // equal x1, equal y1
        assert!(rect.contains(Point::new(1, 10)));
        // inside
        assert!(rect.contains(Point::new(2, 11)));
        // equal x2, equal y2
        assert!(rect.contains(Point::new(3, 12)));
        // greater x2, equal y2
        assert!(!rect.contains(Point::new(4, 12)));
        // equal x2, greater y2
        assert!(!rect.contains(Point::new(3, 13)));
        // greater x2, greater y2
        assert!(!rect.contains(Point::new(4, 13)));
    }
}
