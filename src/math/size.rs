use crate::math::{measurement::ScreenMeasurement, Pixels, Point, Points};

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
