pub type Point = rgx::math::algebra::Point2<f32>;
pub type Rect = rgx::rect::Rect<f32>;

#[derive(Copy, Clone, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Size { width, height }
    }
}

pub trait KludgineRect {
    fn new(origin: Point, size: Size) -> Self;
}

impl KludgineRect for Rect {
    fn new(origin: Point, size: Size) -> Self {
        Rect::sized(origin.x, origin.y, size.width, size.height)
    }
}

pub trait Zeroable {
    fn zero() -> Self;
}

impl Zeroable for Point {
    fn zero() -> Self {
        Self::new(0.0, 0.0)
    }
}
