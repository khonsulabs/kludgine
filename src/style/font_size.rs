use crate::{
    math::{Length, Raw, Scale, Scaled},
    style::{Style, StyleComponent},
};

#[derive(Debug, Copy, Clone)]
pub struct FontSize<Unit: Default + Copy>(pub Length<f32, Unit>);

impl Default for FontSize<Scaled> {
    fn default() -> Self {
        Self::new(14.)
    }
}

impl<Unit: Default + Copy> FontSize<Unit> {
    pub fn new(value: f32) -> Self {
        Self(Length::new(value))
    }

    pub fn get(&self) -> f32 {
        self.0.get()
    }

    pub fn length(&self) -> Length<f32, Unit> {
        self.0
    }
}

impl StyleComponent<Scaled> for FontSize<Scaled> {
    fn scale(&self, scale: Scale<f32, Scaled, Raw>, map: &mut Style<Raw>) {
        map.push(FontSize(self.0 * scale));
    }
}

impl StyleComponent<Raw> for FontSize<Raw> {
    fn scale(&self, _scale: Scale<f32, Raw, Raw>, map: &mut Style<Raw>) {
        map.push(FontSize(self.0));
    }
}
