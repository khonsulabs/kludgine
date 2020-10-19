use crate::{
    math::{Raw, Scaled},
    style::UnscaledStyleComponent,
};

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

impl UnscaledStyleComponent<Raw> for Weight {}
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

impl Into<ttf_parser::Weight> for Weight {
    fn into(self) -> ttf_parser::Weight {
        match self {
            Weight::Thin => ttf_parser::Weight::Thin,
            Weight::ExtraLight => ttf_parser::Weight::ExtraLight,
            Weight::Light => ttf_parser::Weight::Light,
            Weight::Normal => ttf_parser::Weight::Normal,
            Weight::Medium => ttf_parser::Weight::Medium,
            Weight::SemiBold => ttf_parser::Weight::SemiBold,
            Weight::Bold => ttf_parser::Weight::Bold,
            Weight::ExtraBold => ttf_parser::Weight::ExtraBold,
            Weight::Black => ttf_parser::Weight::Black,
            Weight::Other(value) => ttf_parser::Weight::Other(value),
        }
    }
}
