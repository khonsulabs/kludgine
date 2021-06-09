use super::ScreenScale;

pub trait PixelAlignment {
    fn pixel_rounded(&self, scale: ScreenScale) -> Self;
    fn pixel_expanded(&self, scale: ScreenScale) -> Self;
    fn pixel_constrained(&self, scale: ScreenScale) -> Self;
}

impl<T, S> PixelAlignment for T
where
    T: std::ops::Mul<ScreenScale, Output = S> + Copy,
    S: std::ops::Div<ScreenScale, Output = Self>
        + euclid::num::Round
        + euclid::num::Floor
        + euclid::num::Ceil,
{
    fn pixel_rounded(&self, scale: ScreenScale) -> Self {
        let pixels = *self * scale;
        let pixels = pixels.round();
        pixels / scale
    }

    fn pixel_expanded(&self, scale: ScreenScale) -> Self {
        let pixels = *self * scale;
        let pixels = pixels.ceil();
        pixels / scale
    }

    fn pixel_constrained(&self, scale: ScreenScale) -> Self {
        let pixels = *self * scale;
        let pixels = pixels.floor();
        pixels / scale
    }
}
