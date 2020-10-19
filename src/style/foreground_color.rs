use crate::{color::Color, math::Scaled, style::UnscaledStyleComponent};

#[derive(Debug, Clone)]
pub struct ForegroundColor(pub Color);
impl UnscaledStyleComponent<Scaled> for ForegroundColor {}

impl Default for ForegroundColor {
    fn default() -> Self {
        ForegroundColor(Color::BLACK)
    }
}
