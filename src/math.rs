mod measurement;
// mod points;
use approx::relative_eq;
pub use measurement::*;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Rect<S = f32> {
    pub origin: Point<S>,
    pub size: Size<S>,
}

fn float_approximately_ge(a: f32, b: f32) -> bool {
    assert!(!a.is_nan());
    assert!(!b.is_nan());
    if a > b {
        true
    } else {
        relative_eq!(a, b)
    }
}

fn float_approximately_le(a: f32, b: f32) -> bool {
    assert!(!a.is_nan());
    assert!(!b.is_nan());
    if a < b {
        true
    } else {
        relative_eq!(a, b)
    }
}

impl<S> Rect<S>
where
    S: ScreenMeasurement,
{
    pub fn to_points(&self, effective_scale: f32) -> Rect<Points> {
        Rect {
            origin: self.origin.to_points(effective_scale),
            size: self.size.to_points(effective_scale),
        }
    }

    pub fn to_pixels(&self, effective_scale: f32) -> Rect<Pixels> {
        Rect {
            origin: self.origin.to_pixels(effective_scale),
            size: self.size.to_pixels(effective_scale),
        }
    }

    pub fn to_f32(&self) -> Rect<f32> {
        Rect {
            origin: self.origin.to_f32(),
            size: self.size.to_f32(),
        }
    }
}

impl<S> Rect<S>
where
    S: std::ops::Sub<Output = S> + std::ops::Add<Output = S> + Copy + PartialOrd,
{
    pub fn sized(origin: impl Into<Point<S>>, size: impl Into<Size<S>>) -> Self {
        let origin = origin.into();
        let size = size.into();
        Self { origin, size }
    }

    pub fn new(min: impl Into<Point<S>>, max: impl Into<Point<S>>) -> Self {
        let min = min.into();
        let max = max.into();
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

    pub fn coord1(&self) -> Point<S> {
        self.origin
    }

    pub fn coord2(&self) -> Point<S> {
        Point::new(self.x2(), self.y2())
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

    pub fn inset(&self, surround: &Surround<S>) -> Self {
        Self::new(
            Point::new(self.x1() + surround.left, self.y1() + surround.top),
            Point::new(self.x2() - surround.right, self.y2() - surround.bottom),
        )
    }

    pub fn contains(&self, point: &Point<S>) -> bool {
        self.origin.x <= point.x
            && self.x2() >= point.x
            && self.origin.y <= point.y
            && self.y2() >= point.y
    }

    pub fn approximately_contains(&self, point: &Point<S>) -> bool
    where
        S: Into<f32>,
    {
        float_approximately_le(self.origin.x.into(), point.x.into())
            && float_approximately_ge(self.x2().into(), point.x.into())
            && float_approximately_le(self.origin.y.into(), point.y.into())
            && float_approximately_ge(self.y2().into(), point.y.into())
    }

    pub fn contains_rect(&self, rect: &Rect<S>) -> bool {
        self.contains(&rect.coord1()) && self.contains(&rect.coord2())
    }

    pub fn approximately_contains_rect(&self, rect: &Rect<S>) -> bool
    where
        S: Into<f32>,
    {
        self.approximately_contains(&rect.coord1()) && self.approximately_contains(&rect.coord2())
    }

    pub fn area(&self) -> S
    where
        S: std::ops::Mul<Output = S> + Copy,
    {
        self.size.area()
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

impl<S> std::ops::Mul<S> for Rect<S>
where
    S: std::ops::Mul<Output = S> + Copy,
{
    type Output = Self;

    fn mul(self, s: S) -> Self {
        Self {
            origin: self.origin * s,
            size: self.size * s,
        }
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

    pub fn to_points(&self, effective_scale: f32) -> Point<Points>
    where
        S: ScreenMeasurement,
    {
        Point {
            x: self.x.to_points(effective_scale),
            y: self.y.to_points(effective_scale),
        }
    }

    pub fn to_pixels(&self, effective_scale: f32) -> Point<Pixels>
    where
        S: ScreenMeasurement,
    {
        Point {
            x: self.x.to_pixels(effective_scale),
            y: self.y.to_pixels(effective_scale),
        }
    }

    pub fn to_f32(&self) -> Point<f32>
    where
        S: ScreenMeasurement,
    {
        Point {
            x: self.x.to_f32(),
            y: self.y.to_f32(),
        }
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

impl<S> std::ops::Add<Point<S>> for Point<S>
where
    S: std::ops::Add<Output = S> + Copy,
{
    type Output = Self;

    fn add(self, s: Self) -> Self {
        Self {
            x: self.x + s.x,
            y: self.y + s.y,
        }
    }
}

impl<S> std::ops::Sub<Point<S>> for Point<S>
where
    S: std::ops::Sub<Output = S> + Copy,
{
    type Output = Self;

    fn sub(self, s: Self) -> Self {
        Self {
            x: self.x - s.x,
            y: self.y - s.y,
        }
    }
}

impl<S> Into<rgx::math::Point2<S>> for Point<S> {
    fn into(self) -> rgx::math::Point2<S> {
        rgx::math::Point2::new(self.x, self.y)
    }
}

impl<S> From<rgx::math::Point2<S>> for Point<S> {
    fn from(pt: rgx::math::Point2<S>) -> Self {
        Self::new(pt.x, pt.y)
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct Size<S = f32> {
    pub width: S,
    pub height: S,
}

impl<S> Size<S> {
    pub const fn new(width: S, height: S) -> Self {
        Self { width, height }
    }

    pub fn area(&self) -> S
    where
        S: std::ops::Mul<Output = S> + Copy,
    {
        self.width * self.height
    }

    pub fn to_points(&self, effective_scale: f32) -> Size<Points>
    where
        S: ScreenMeasurement,
    {
        Size {
            width: self.width.to_points(effective_scale),
            height: self.height.to_points(effective_scale),
        }
    }

    pub fn to_pixels(&self, effective_scale: f32) -> Size<Pixels>
    where
        S: ScreenMeasurement,
    {
        Size {
            width: self.width.to_pixels(effective_scale),
            height: self.height.to_pixels(effective_scale),
        }
    }

    pub fn to_f32(&self) -> Size<f32>
    where
        S: ScreenMeasurement,
    {
        Size {
            width: self.width.to_f32(),
            height: self.height.to_f32(),
        }
    }
}

impl From<Size<u32>> for Size<f32> {
    fn from(value: Size<u32>) -> Self {
        Self {
            width: value.width as f32,
            height: value.height as f32,
        }
    }
}

impl From<Size<f32>> for Size<u32> {
    fn from(value: Size<f32>) -> Self {
        Self {
            width: value.width as u32,
            height: value.height as u32,
        }
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

impl<S, T> std::ops::Div<T> for Size<S>
where
    S: std::ops::Div<T, Output = S> + Copy,
    T: Copy,
{
    type Output = Self;

    fn div(self, t: T) -> Self {
        Size {
            width: self.width / t,
            height: self.height / t,
        }
    }
}

impl<S, T> std::ops::Mul<S> for Size<S>
where
    S: std::ops::Mul<Output = T> + Copy,
{
    type Output = Size<T>;

    fn mul(self, s: S) -> Size<T> {
        Size {
            width: self.width * s,
            height: self.height * s,
        }
    }
}

// impl<S> std::ops::Div<Size<S>> for Size<S>
// where
//     S: std::ops::Div<S, Output = S> + Copy,
// {
//     type Output = Self;

//     fn div(self, s: Size<S>) -> Self {
//         Self {
//             width: self.width / s.width,
//             height: self.height / s.height,
//         }
//     }
// }

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

impl<S> std::ops::Sub<Size<S>> for Size<Option<S>>
where
    S: std::ops::Sub<Output = S> + Copy,
{
    type Output = Self;

    fn sub(self, s: Size<S>) -> Self {
        Self {
            width: self.width.map(|w| w - s.width),
            height: self.height.map(|h| h - s.height),
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

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Dimension {
    Auto,
    /// Scale-corrected to the users preference of DPI
    Points(Points),
}

impl Dimension {
    pub fn from_points(value: impl Into<Points>) -> Self {
        Self::Points(value.into())
    }

    pub fn is_auto(&self) -> bool {
        self == &Dimension::Auto
    }
    pub fn is_points(&self) -> bool {
        !self.is_auto()
    }

    pub fn points(&self) -> Option<Points> {
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

impl From<Points> for Dimension {
    fn from(value: Points) -> Self {
        Dimension::from_points(value)
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
        let rect = Rect::<i32>::new(Point::new(1, 10), Point::new(3, 12));
        // lower x, lower y
        assert!(!rect.contains(&Point::new(0, 9)));
        // lower x, equal y
        assert!(!rect.contains(&Point::new(0, 10)));
        // equal x, lower y
        assert!(!rect.contains(&Point::new(1, 9)));
        // equal x1, equal y1
        assert!(rect.contains(&Point::new(1, 10)));
        // inside
        assert!(rect.contains(&Point::new(2, 11)));
        // equal x2, equal y2
        assert!(rect.contains(&Point::new(3, 12)));
        // greater x2, equal y2
        assert!(!rect.contains(&Point::new(4, 12)));
        // equal x2, greater y2
        assert!(!rect.contains(&Point::new(3, 13)));
        // greater x2, greater y2
        assert!(!rect.contains(&Point::new(4, 13)));
    }

    #[test]
    fn test_rect_approx_contains() {
        let rect = Rect::<f32>::new(Point::new(1., 10.), Point::new(3., 12.));
        // lower x, lower y
        assert!(!rect.approximately_contains(&Point::new(1., 9.)));
        // lower x, equal y
        assert!(!rect.approximately_contains(&Point::new(0., 10.)));
        // equal x, lower y
        assert!(!rect.approximately_contains(&Point::new(1., 9.)));
        // equal x1, equal y1
        assert!(rect.approximately_contains(&Point::new(1., 10.)));
        // inside
        assert!(rect.approximately_contains(&Point::new(2., 11.)));
        // equal x2, equal y2
        assert!(rect.approximately_contains(&Point::new(3., 12.)));
        // greater x2, equal y2
        assert!(!rect.approximately_contains(&Point::new(4., 12.)));
        // equal x2, greater y2
        assert!(!rect.approximately_contains(&Point::new(3., 13.)));
        // greater x2, greater y2
        assert!(!rect.approximately_contains(&Point::new(4., 13.)));
    }
}
