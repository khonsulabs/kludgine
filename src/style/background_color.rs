use crate::{color::Color, math::Scaled, style::UnscaledStyleComponent};

#[derive(Debug, Clone)]
pub struct BackgroundColor(pub Color);
impl UnscaledStyleComponent<Scaled> for BackgroundColor {}

impl Default for BackgroundColor {
    fn default() -> Self {
        BackgroundColor(Color::WHITE)
    }
}
