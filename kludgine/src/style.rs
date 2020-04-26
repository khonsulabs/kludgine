use crate::{
    math::{Dimension, Point, Size, Surround},
    scene::SceneTarget,
};
pub use rgx::color::Rgba as Color;
pub use ttf_parser::Weight;

#[derive(Default, Clone, Debug)]
pub struct Layout {
    pub location: Point,
    pub margin: Surround<Dimension>,
    pub padding: Surround<Dimension>,
    pub border: Surround<Dimension>,
    pub min_size: Size<Dimension>,
    pub max_size: Size<Dimension>,
}

#[derive(Default, Clone, Debug)]
pub struct Style {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: Option<Weight>,
    pub color: Option<Color>,
}

impl Style {
    pub fn inherit_from(&self, parent: &Style) -> Self {
        Self {
            font_family: self.font_family.clone().or(parent.font_family.clone()),
            font_size: self.font_size.or(parent.font_size),
            font_weight: self.font_weight.or(parent.font_weight),
            color: self.color.or(parent.color),
        }
    }

    pub fn effective_style(&self, scene: &mut SceneTarget) -> EffectiveStyle {
        EffectiveStyle {
            font_family: self
                .font_family
                .clone()
                .unwrap_or_else(|| "sans-serif".to_owned()),
            font_size: self.font_size.unwrap_or(14.0) * scene.effective_scale_factor(),
            font_weight: self.font_weight.unwrap_or(Weight::Normal),
            color: self.color.unwrap_or(Color::BLACK),
        }
    }
}

#[derive(PartialEq, Clone, Debug, Default)]
pub struct EffectiveStyle {
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: Weight,
    pub color: Color,
}
