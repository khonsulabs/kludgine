use crate::{
    math::{Points, Scaled, Surround},
    style::{BackgroundColor, Style, TextColor},
    theme::{Palette, Theme},
    ui::{ControlBackgroundColor, ControlPadding, TextFieldBackgroundColor},
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
                self.palette.light.default.background.normal(),
            ))
            .with(ControlBackgroundColor(
                self.palette.light.control.background.normal(),
            ))
            .with(ControlPadding(Surround::uniform(Points::new(10.))))
    }

    fn default_hover_style(&self) -> Style<Scaled> {
        Style::new()
            .with(TextColor(self.palette.light.control.text.normal()))
            .with(BackgroundColor(
                self.palette.light.default.background.lighter(),
            ))
            .with(ControlBackgroundColor(
                self.palette.light.control.background.lighter(),
            ))
            .with(ControlPadding(Surround::uniform(Points::new(10.))))
    }

    fn default_active_style(&self) -> Style<Scaled> {
        Style::new()
            .with(TextColor(self.palette.light.control.text.normal()))
            .with(BackgroundColor(
                self.palette.light.default.background.darker(),
            ))
            .with(ControlBackgroundColor(
                self.palette.light.control.background.darker(),
            ))
            .with(TextFieldBackgroundColor(
                self.palette.light.control.background.normal(),
            ))
            .with(ControlPadding(Surround::uniform(Points::new(10.))))
    }
}
