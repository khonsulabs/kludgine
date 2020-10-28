use crate::{math::Scaled, style::UnscaledStyleComponent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}
impl UnscaledStyleComponent<Scaled> for Alignment {}

impl Default for Alignment {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}
impl UnscaledStyleComponent<Scaled> for VerticalAlignment {}

impl Default for VerticalAlignment {
    fn default() -> Self {
        Self::Top
    }
}
