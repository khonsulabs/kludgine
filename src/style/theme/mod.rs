use crate::{
    color::Color,
    math::Scaled,
    style::{Style, StyleSheet},
};
use std::collections::HashMap;
mod minimal;
mod selector;
pub use self::{
    minimal::Minimal,
    selector::{Classes, Id, Selector},
};
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

#[derive(Debug)]
pub struct Theme {
    pub(crate) default_font_family: String,
    default: Option<Style<Scaled>>,
    rules: Vec<ThemeRule>,
}

impl Theme {
    pub fn new(default_font_family: String, base_style: Style<Scaled>) -> Self {
        Self {
            default_font_family,
            default: Some(base_style),
            rules: Vec::new(),
        }
    }

    pub fn with_font_family(default_font_family: String) -> Self {
        Self::new(default_font_family, Default::default())
    }

    pub fn when<
        P: Fn(ThemeBuilderContext) -> ThemeRulePredicate,
        F: Fn(Style<Scaled>) -> Style<Scaled>,
    >(
        mut self,
        predicate: P,
        style_builder: F,
    ) -> Self {
        self.rules.push(ThemeRule {
            predicate: predicate(ThemeBuilderContext::default()),
            style: style_builder(Style::default()),
        });
        self
    }

    pub fn stylesheet_for(&self, id: Option<&Id>, classes: Option<&Classes>) -> StyleSheet {
        StyleSheet {
            normal: self.style_for(id, classes, &StyleState::Normal),
            active: self.style_for(id, classes, &StyleState::Active),
            focus: self.style_for(id, classes, &StyleState::Focus),
            hover: self.style_for(id, classes, &StyleState::Hover),
        }
    }

    fn style_for(
        &self,
        id: Option<&Id>,
        classes: Option<&Classes>,
        state: &StyleState,
    ) -> Style<Scaled> {
        let mut style = Style::default();

        for rule in self.rules.iter().rev() {
            if rule.predicate.matches(id, classes, state) {
                style = style.merge_with(&rule.style, false);
            }
        }

        style
    }
}

enum StyleState {
    Normal,
    Hover,
    Active,
    Focus,
}

#[derive(Debug)]
struct ThemeRule {
    predicate: ThemeRulePredicate,
    style: Style<Scaled>,
}

#[derive(Default)]
pub struct ThemeBuilderContext {
    pub classes: ThemeBuilderClasses,
    pub id: ThemeBuilderId,
}

impl ThemeBuilderContext {
    pub fn is_active(&self) -> ThemeRulePredicate {
        ThemeRulePredicate::IsActive
    }

    pub fn is_hovered(&self) -> ThemeRulePredicate {
        ThemeRulePredicate::IsHovered
    }

    pub fn is_focused(&self) -> ThemeRulePredicate {
        ThemeRulePredicate::IsFocused
    }
}

#[derive(Debug)]
pub enum ThemeRulePredicate {
    ClassesContains(Selector),
    IdEquals(Selector),
    Not(Box<ThemeRulePredicate>),
    And(Box<ThemeRulePredicate>, Box<ThemeRulePredicate>),
    Or(Box<ThemeRulePredicate>, Box<ThemeRulePredicate>),
    IsActive,
    IsHovered,
    IsFocused,
}

impl ThemeRulePredicate {
    pub fn and(self, other: ThemeRulePredicate) -> ThemeRulePredicate {
        ThemeRulePredicate::And(Box::new(self), Box::new(other))
    }

    pub fn or(self, other: ThemeRulePredicate) -> ThemeRulePredicate {
        ThemeRulePredicate::Or(Box::new(self), Box::new(other))
    }

    fn matches(&self, id: Option<&Id>, classes: Option<&Classes>, state: &StyleState) -> bool {
        match self {
            ThemeRulePredicate::ClassesContains(comparison) => {
                if let Some(classes) = classes {
                    classes.0.contains(comparison)
                } else {
                    false
                }
            }
            ThemeRulePredicate::IdEquals(comparison) => {
                if let Some(id) = id {
                    &id.0 == comparison
                } else {
                    false
                }
            }
            ThemeRulePredicate::Not(a) => !a.matches(id, classes, state),
            ThemeRulePredicate::And(a, b) => {
                a.matches(id, classes, state) && b.matches(id, classes, state)
            }
            ThemeRulePredicate::Or(a, b) => {
                a.matches(id, classes, state) || b.matches(id, classes, state)
            }
            ThemeRulePredicate::IsActive => matches!(state, StyleState::Active),
            ThemeRulePredicate::IsHovered => matches!(state, StyleState::Hover),
            ThemeRulePredicate::IsFocused => matches!(state, StyleState::Focus),
        }
    }
}

#[derive(Default)]
pub struct ThemeBuilderClasses;

impl ThemeBuilderClasses {
    pub fn contains(&self, class: &str) -> ThemeRulePredicate {
        ThemeRulePredicate::ClassesContains(Selector::from(class))
    }
}

#[derive(Default)]
pub struct ThemeBuilderId;

impl ThemeBuilderId {
    pub fn eq(&self, id: &str) -> ThemeRulePredicate {
        ThemeRulePredicate::IdEquals(Selector::from(id))
    }
}

impl std::ops::Not for ThemeRulePredicate {
    type Output = Self;

    fn not(self) -> Self::Output {
        ThemeRulePredicate::Not(Box::new(self))
    }
}
