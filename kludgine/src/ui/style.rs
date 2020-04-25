use crate::math::Size;
pub use rgx::color::Rgba;
pub use ttf_parser::Weight;

#[derive(Copy, Clone, PartialEq)]
pub enum Dimension {
    Auto,
    /// Scale-corrected to the users preference of DPI
    Points(f32),
}

impl Default for Dimension {
    fn default() -> Self {
        Dimension::Auto
    }
}

#[derive(Default, Clone)]
pub struct Layout {
    pub min_size: Size<Dimension>,
    pub max_size: Size<Dimension>,
}
#[derive(Default, Clone)]
pub struct Style {
    pub font: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: Option<Weight>,
    pub color: Option<Rgba>,
}

impl Style {
    pub fn inherit_from(&self, parent: &Style) -> Self {
        Self {
            font: self.font.clone().or(parent.font.clone()),
            font_size: self.font_size.or(parent.font_size),
            font_weight: self.font_weight.or(parent.font_weight),
            color: self.color.or(parent.color),
        }
    }

    pub fn effective_style(&self) -> EffectiveStyle {
        EffectiveStyle {
            font_family: self.font.clone().unwrap_or_else(|| "sans-serif".to_owned()),
            font_size: self.font_size.unwrap_or(14.0),
            font_weight: self.font_weight.unwrap_or(Weight::Normal),
            color: self.color.unwrap_or(Rgba::BLACK),
        }
    }
}

pub struct EffectiveStyle {
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: Weight,
    pub color: Rgba,
}
