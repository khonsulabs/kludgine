use std::fmt::Debug;

use crate::{
    color::Color,
    math::Scaled,
    style::{GenericStyle, UnscaledFallbackStyle, UnscaledStyleComponent},
    theme::SystemTheme,
};

#[derive(Debug, Clone, Copy)]
pub struct ColorPair {
    pub light_color: Color,
    pub dark_color: Color,
}

impl From<Color> for ColorPair {
    fn from(color: Color) -> Self {
        Self {
            light_color: color,
            dark_color: color,
        }
    }
}

impl ColorPair {
    pub fn themed_color(&self, system_theme: &SystemTheme) -> Color {
        match system_theme {
            SystemTheme::Light => self.light_color,
            SystemTheme::Dark => self.dark_color,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ForegroundColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for ForegroundColor {}

impl Default for ForegroundColor {
    fn default() -> Self {
        ForegroundColor(ColorPair {
            light_color: Color::BLACK,
            dark_color: Color::WHITE,
        })
    }
}

impl UnscaledFallbackStyle for ForegroundColor {}

impl Into<ColorPair> for ForegroundColor {
    fn into(self) -> ColorPair {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct BackgroundColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for BackgroundColor {
    fn unscaled_should_be_inherited(&self) -> bool {
        false
    }
}
impl UnscaledFallbackStyle for BackgroundColor {}

impl Default for BackgroundColor {
    fn default() -> Self {
        BackgroundColor(ColorPair {
            light_color: Color::WHITE,
            dark_color: Color::BLACK,
        })
    }
}

impl Into<ColorPair> for BackgroundColor {
    fn into(self) -> ColorPair {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct TextColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for TextColor {}

impl Default for TextColor {
    fn default() -> Self {
        Self(ForegroundColor::default().0)
    }
}

impl UnscaledFallbackStyle for TextColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ForegroundColor::lookup_unscaled(style).map(|fg| TextColor(fg.0)))
    }
}

impl Into<ColorPair> for TextColor {
    fn into(self) -> ColorPair {
        self.0
    }
}
