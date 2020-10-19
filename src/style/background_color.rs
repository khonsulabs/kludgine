use crate::{
    color::Color,
    math::{Raw, Scaled},
    style::UnscaledStyleComponent,
};

#[derive(Debug, Clone)]
pub struct BackgroundColor(pub Color);
impl UnscaledStyleComponent<Raw> for BackgroundColor {}
impl UnscaledStyleComponent<Scaled> for BackgroundColor {}

impl Default for BackgroundColor {
    fn default() -> Self {
        BackgroundColor(Color::WHITE)
    }
}
