use crate::{color::Color, math::Scaled, style::UnscaledStyleComponent};

use std::fmt::Debug;
use winit::window::Theme;

#[derive(Debug, Clone, Default, Copy)]
pub struct ColorPair {
    pub light_color: Color,
    pub dark_color: Color,
}

impl ColorPair {
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.light_color = self.light_color.with_alpha(alpha);
        self.dark_color = self.dark_color.with_alpha(alpha);
        self
    }
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
    pub fn themed_color(&self, system_theme: &Theme) -> Color {
        match system_theme {
            Theme::Light => self.light_color,
            Theme::Dark => self.dark_color,
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

impl From<ForegroundColor> for ColorPair {
    fn from(color: ForegroundColor) -> Self {
        color.0
    }
}

#[derive(Debug, Clone)]
pub struct BackgroundColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for BackgroundColor {
    fn unscaled_should_be_inherited(&self) -> bool {
        false
    }
}

impl Default for BackgroundColor {
    fn default() -> Self {
        BackgroundColor(ColorPair {
            light_color: Color::WHITE,
            dark_color: Color::BLACK,
        })
    }
}

impl From<BackgroundColor> for ColorPair {
    fn from(color: BackgroundColor) -> Self {
        color.0
    }
}
