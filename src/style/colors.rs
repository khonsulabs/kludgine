use std::fmt::Debug;

use crate::{
    color::Color,
    math::Scaled,
    style::{GenericStyle, UnscaledFallbackStyle, UnscaledStyleComponent},
};

#[derive(Debug, Clone)]
pub struct ForegroundColor(pub Color);
impl UnscaledStyleComponent<Scaled> for ForegroundColor {}

impl Default for ForegroundColor {
    fn default() -> Self {
        ForegroundColor(Color::BLACK)
    }
}

impl UnscaledFallbackStyle for ForegroundColor {}

#[derive(Debug, Clone)]
pub struct BackgroundColor(pub Color);
impl UnscaledStyleComponent<Scaled> for BackgroundColor {}
impl UnscaledFallbackStyle for BackgroundColor {}

impl Default for BackgroundColor {
    fn default() -> Self {
        BackgroundColor(Color::WHITE)
    }
}

impl Into<Color> for BackgroundColor {
    fn into(self) -> Color {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct TextColor(pub Color);
impl UnscaledStyleComponent<Scaled> for TextColor {}

impl UnscaledFallbackStyle for TextColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ForegroundColor::lookup_unscaled(style).map(|fg| TextColor(fg.0)))
    }
}

impl Into<Color> for TextColor {
    fn into(self) -> Color {
        self.0
    }
}
