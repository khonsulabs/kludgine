use crate::{
    math::{Points, Scaled, Surround},
    style::{
        theme::{Palette, Theme},
        BackgroundColor, ColorPair, Style, TextColor,
    },
    ui::{
        Border, ComponentBorder, ControlBackgroundColor, ControlPadding, TextFieldBackgroundColor,
        TextFieldBorder,
    },
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
            .with(TextColor(ColorPair {
                light_color: self.palette.light.control.text.normal(),
                dark_color: self.palette.dark.control.text.normal(),
            }))
            .with(BackgroundColor(ColorPair {
                light_color: self.palette.light.default.background.normal(),
                dark_color: self.palette.dark.default.background.normal(),
            }))
            .with(ControlBackgroundColor(ColorPair {
                light_color: self.palette.light.control.background.normal(),
                dark_color: self.palette.dark.control.background.normal(),
            }))
            .with(ControlPadding(Surround::uniform(Points::new(10.))))
            .with(TextFieldBorder(ComponentBorder::uniform(Border::new(
                2.,
                ColorPair {
                    light_color: self.palette.light.control.background.darker(),
                    dark_color: self.palette.dark.control.background.lighter(),
                },
            ))))
    }

    fn default_hover_style(&self) -> Style<Scaled> {
        self.default_normal_style()
            .with(ControlBackgroundColor(ColorPair {
                light_color: self.palette.light.control.background.lighter(),
                dark_color: self.palette.dark.control.background.lighter(),
            }))
            .with(ControlPadding(Surround::uniform(Points::new(10.))))
    }

    fn default_active_style(&self) -> Style<Scaled> {
        Style::new()
            .with(TextColor(ColorPair {
                light_color: self.palette.light.control.text.normal(),
                dark_color: self.palette.dark.control.background.normal(),
            }))
            .with(ControlBackgroundColor(ColorPair {
                light_color: self.palette.light.control.background.darker(),
                dark_color: self.palette.dark.control.background.darker(),
            }))
            .with(TextFieldBackgroundColor(ColorPair {
                light_color: self.palette.light.control.background.normal(),
                dark_color: self.palette.dark.control.background.normal(),
            }))
            .with(ControlPadding(Surround::uniform(Points::new(10.))))
    }

    fn default_focus_style(&self) -> Style<Scaled> {
        self.default_normal_style()
            .with(TextFieldBorder(ComponentBorder::uniform(Border::new(
                2.,
                self.palette.primary.normal().into(),
            ))))
    }
}
