use crate::{
    color::Color,
    math::Scaled,
    style::{Style, StyleSheet},
};
use std::collections::HashMap;
mod minimal;
pub use minimal::Minimal;
pub use winit::window::Theme as SystemTheme;

pub enum ElementKind {
    Button,
    Label,
}

#[derive(Debug, Clone)]
pub struct ColorGroup {
    pub text: VariableColor,
    pub background: VariableColor,
}

#[derive(Debug)]
pub struct Palette {
    pub dark: PaletteShade,
    pub light: PaletteShade,

    pub primary: VariableColor,
    pub danger: VariableColor,
    pub warning: VariableColor,
    pub info: VariableColor,
    pub success: VariableColor,

    pub others: HashMap<String, VariableColor>,
}

#[derive(Debug)]
pub struct PaletteShade {
    pub default: ColorGroup,
    pub control: ColorGroup,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            dark: PaletteShade {
                default: ColorGroup {
                    text: Color::WHITE.into(),
                    background: Color::BLACK.into(),
                },
                control: ColorGroup {
                    text: Color::WHITE.into(),
                    background: Color::new(0.3, 0.3, 0.3, 1.).into(),
                },
            },
            light: PaletteShade {
                default: ColorGroup {
                    text: Color::BLACK.into(),
                    background: Color::WHITE.into(),
                },
                control: ColorGroup {
                    text: Color::BLACK.into(),
                    background: Color::new(0.7, 0.7, 0.7, 1.).into(),
                },
            },
            primary: Color::ORANGE.into(),
            danger: Color::RED.into(),
            warning: Color::YELLOW.into(),
            success: Color::GREEN.into(),
            info: Color::BLUE.into(),
            others: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Intent {
    Default,
    Primary,
    Danger,
    Warning,
    Info,
    Other(String),
}

impl Default for Intent {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone)]
pub enum VariableColor {
    Auto(Color),
    Manual {
        lighter: Color,
        normal: Color,
        darker: Color,
    },
}

impl VariableColor {
    pub fn lighter(&self) -> Color {
        match self {
            VariableColor::Auto(base) => base.lighten(0.3),
            VariableColor::Manual { lighter, .. } => *lighter,
        }
    }

    pub fn normal(&self) -> Color {
        match self {
            VariableColor::Auto(base) => *base,
            VariableColor::Manual { normal, .. } => *normal,
        }
    }

    pub fn darker(&self) -> Color {
        match self {
            VariableColor::Auto(base) => base.darken(0.3),
            VariableColor::Manual { darker, .. } => *darker,
        }
    }
}

impl From<Color> for VariableColor {
    fn from(color: Color) -> Self {
        VariableColor::Auto(color)
    }
}

pub trait Theme: Send + Sync {
    fn default_font_family(&self) -> &'_ str;

    fn default_normal_style(&self) -> Style<Scaled>;

    fn default_active_style(&self) -> Style<Scaled> {
        self.default_normal_style()
    }

    fn default_hover_style(&self) -> Style<Scaled> {
        self.default_normal_style()
    }

    fn default_focus_style(&self) -> Style<Scaled> {
        self.default_normal_style()
    }

    fn default_style_sheet(&self) -> StyleSheet {
        StyleSheet {
            normal: self.default_normal_style(),
            active: self.default_active_style(),
            hover: self.default_hover_style(),
            focus: self.default_focus_style(),
        }
    }
}
