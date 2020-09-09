use crate::math::{measurement::ScreenMeasurement, Pixels, Points};
#[derive(Copy, Clone, Default, Debug, PartialEq, Hash, Eq)]
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

impl<S> std::ops::Add<S> for Point<S>
where
    S: std::ops::Add<Output = S> + Copy,
{
    type Output = Self;

    fn add(self, s: S) -> Self {
        Self {
            x: self.x + s,
            y: self.y + s,
        }
    }
}

impl<S> std::ops::Sub<S> for Point<S>
where
    S: std::ops::Sub<Output = S> + Copy,
{
    type Output = Self;

    fn sub(self, s: S) -> Self {
        Self {
            x: self.x - s,
            y: self.y - s,
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
