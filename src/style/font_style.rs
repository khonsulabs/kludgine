use crate::{
    math::{Raw, Scaled},
    style::UnscaledStyleComponent,
};

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum FontStyle {
    Regular,
    Italic,
    Oblique,
}

impl Default for FontStyle {
    fn default() -> Self {
        FontStyle::Regular
    }
}

impl UnscaledStyleComponent<Raw> for FontStyle {}
impl UnscaledStyleComponent<Scaled> for FontStyle {}
