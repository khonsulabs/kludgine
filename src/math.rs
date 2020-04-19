pub type Point<S = f32> = rgx::math::algebra::Point2<S>;
pub type Rect<S = f32> = rgx::rect::Rect<S>;

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
