use crate::math::{measurement::ScreenMeasurement, Pixels, Point, Points, Size, Surround};
use approx::relative_eq;

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

    pub fn coord1(&self) -> Point<S> {
        self.origin
    }

    pub fn x2(&self) -> S {
        self.origin.x + self.size.width
    }

    pub fn y2(&self) -> S {
        self.origin.y + self.size.height
    }

    pub fn coord2(&self) -> Point<S> {
        Point::new(self.x2(), self.y2())
    }

    pub fn x1y1(&self) -> Point<S> {
        Point::new(self.x1(), self.y1())
    }

    pub fn x1y2(&self) -> Point<S> {
        Point::new(self.x1(), self.y2())
    }

    pub fn x2y1(&self) -> Point<S> {
        Point::new(self.x2(), self.y1())
    }

    pub fn x2y2(&self) -> Point<S> {
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

    pub fn intersects_with(&self, other: &Rect<S>) -> bool {
        let has_no_overlap = self.origin.x + self.size.width < other.origin.x
            || other.origin.x + other.size.width < self.origin.x
            || self.origin.y + self.size.height < other.origin.y
            || other.origin.y + other.size.height < self.origin.y;

        !has_no_overlap
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

impl<S> Rect<S>
where
    S: std::ops::Div<f32, Output = S>
        + std::ops::Add<Output = S>
        + Copy,
{
    pub fn center(&self) -> Point<S> {
        let half_size = self.size / 2f32;
        self.origin + half_size
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
