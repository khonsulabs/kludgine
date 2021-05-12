use crate::{math::Scaled, style::UnscaledStyleComponent};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Weight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
    Other(u16),
}

impl Default for Weight {
    fn default() -> Self {
        ttf_parser::Weight::default().into()
    }
}

impl UnscaledStyleComponent<Scaled> for Weight {}

impl Weight {
    pub fn to_number(self) -> u16 {
        let ttf: ttf_parser::Weight = self.into();
        ttf.to_number()
    }
}

impl From<ttf_parser::Weight> for Weight {
    fn from(weight: ttf_parser::Weight) -> Self {
        match weight {
            ttf_parser::Weight::Thin => Weight::Thin,
            ttf_parser::Weight::ExtraLight => Weight::ExtraLight,
            ttf_parser::Weight::Light => Weight::Light,
            ttf_parser::Weight::Normal => Weight::Normal,
            ttf_parser::Weight::Medium => Weight::Medium,
            ttf_parser::Weight::SemiBold => Weight::SemiBold,
            ttf_parser::Weight::Bold => Weight::Bold,
            ttf_parser::Weight::ExtraBold => Weight::ExtraBold,
            ttf_parser::Weight::Black => Weight::Black,
            ttf_parser::Weight::Other(value) => Weight::Other(value),
        }
    }
}

impl From<Weight> for ttf_parser::Weight {
    fn from(weight: Weight) -> Self {
        match weight {
            Weight::Thin => Self::Thin,
            Weight::ExtraLight => Self::ExtraLight,
            Weight::Light => Self::Light,
            Weight::Normal => Self::Normal,
            Weight::Medium => Self::Medium,
            Weight::SemiBold => Self::SemiBold,
            Weight::Bold => Self::Bold,
            Weight::ExtraBold => Self::ExtraBold,
            Weight::Black => Self::Black,
            Weight::Other(value) => Self::Other(value),
        }
    }
}
