pub type Point<S = f32> = rgx::math::algebra::Point2<S>;
pub type Rect<S = f32> = rgx::rect::Rect<S>;

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
}

impl KludgineRect for Rect {
    fn new(origin: Point, size: Size) -> Self {
        Rect::sized(origin.x, origin.y, size.width, size.height)
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
