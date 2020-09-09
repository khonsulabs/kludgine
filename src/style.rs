use crate::{
    color::Color,
    math::{Pixels, Points},
    scene::SceneTarget,
};
pub use ttf_parser::Weight;

#[derive(Default, Clone, Debug)]
pub struct Style {
    pub font_family: Option<String>,
    pub font_size: Option<Points>,
    pub font_weight: Option<Weight>,
    pub color: Option<Color>,
    pub background_color: Option<Color>,
    pub alignment: Option<Alignment>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

impl Default for Alignment {
    fn default() -> Self {
        Self::Left
    }
}

impl Style {
    pub fn inherit_from(&self, parent: &Style) -> Self {
        Self {
            font_family: self
                .font_family
                .clone()
                .or_else(|| parent.font_family.clone()),
            font_size: self.font_size.or(parent.font_size),
            font_weight: self.font_weight.or(parent.font_weight),
            color: self.color.or(parent.color),
            background_color: self.background_color.or(parent.background_color),
            alignment: self.alignment.or(parent.alignment),
        }
    }

    pub async fn effective_style(&self, scene: &SceneTarget) -> EffectiveStyle {
        EffectiveStyle {
            font_family: self
                .font_family
                .clone()
                .unwrap_or_else(|| "sans-serif".to_owned()),
            font_size: self.font_size.unwrap_or_else(|| Points::new(14.0))
                * scene.effective_scale_factor().await,
            font_weight: self.font_weight.unwrap_or(Weight::Normal),
            color: self.color.unwrap_or(Color::BLACK),
            background_color: self.background_color,
            alignment: self.alignment.unwrap_or_default(),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct StyleSheet {
    pub normal: Style,
    pub hover: Style,
    pub focus: Style,
    pub active: Style,
}

impl From<Style> for StyleSheet {
    fn from(style: Style) -> Self {
        Self {
            normal: style.clone(),
            active: style.clone(),
            hover: style.clone(),
            focus: style,
        }
    }
}

#[derive(PartialEq, Clone, Debug, Default)]
pub struct EffectiveStyle {
    pub font_family: String,
    pub font_size: Pixels,
    pub font_weight: Weight,
    pub color: Color,
    pub background_color: Option<Color>,
    pub alignment: Alignment,
}
