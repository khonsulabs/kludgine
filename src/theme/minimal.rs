use crate::{
    math::Scaled,
    style::{BackgroundColor, Style, TextColor},
    theme::{Palette, Theme},
};

#[derive(Debug)]
pub struct Minimal {
    font_family: String,
    palette: Palette,
}

impl Minimal {
    pub fn new<S: ToString>(font_family: S, palette: Palette) -> Self {
        Self {
            font_family: font_family.to_string(),
            palette,
        }
    }
}

impl Default for Minimal {
    fn default() -> Self {
        Self::new("Roboto", Default::default())
    }
}

impl Theme for Minimal {
    fn default_font_family(&self) -> &'_ str {
        &self.font_family
    }

    fn default_normal_style(&self) -> Style<Scaled> {
        Style::new()
            .with(TextColor(self.palette.light.control.text.normal()))
            .with(BackgroundColor(
                self.palette.light.control.background.normal(),
            ))
    }

    fn default_hover_style(&self) -> Style<Scaled> {
        Style::new()
            .with(TextColor(self.palette.light.control.text.normal()))
            .with(BackgroundColor(
                self.palette.light.control.background.lighter(),
            ))
    }

    fn default_active_style(&self) -> Style<Scaled> {
        Style::new()
            .with(TextColor(self.palette.light.control.text.normal()))
            .with(BackgroundColor(
                self.palette.light.control.background.darker(),
            ))
    }
}
