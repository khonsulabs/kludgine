use crate::{
    math::Scaled,
    style::{BackgroundColor, Style, TextColor},
    theme::{Palette, Theme},
};

#[derive(Debug, Default)]
pub struct Minimal {
    palette: Palette,
}

impl Minimal {
    pub fn new(palette: Palette) -> Self {
        Self { palette }
    }
}

impl Theme for Minimal {
    fn default_font_family(&self) -> &'_ str {
        "Roboto"
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
