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

impl UnscaledFallbackStyle for ForegroundColor {
    fn lookup(style: GenericStyle) -> Option<Self> {
        style.get::<Self>().cloned()
    }
}

#[derive(Debug, Clone)]
pub struct BackgroundColor(pub Color);
impl UnscaledStyleComponent<Scaled> for BackgroundColor {}

impl Default for BackgroundColor {
    fn default() -> Self {
        BackgroundColor(Color::WHITE)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TextColor(pub Color);
impl UnscaledStyleComponent<Scaled> for TextColor {}

impl UnscaledFallbackStyle for TextColor {
    fn lookup(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ForegroundColor::lookup(style).map(|fg| TextColor(fg.0)))
    }
}
