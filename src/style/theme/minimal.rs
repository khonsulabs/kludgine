use crate::{
    math::{Points, Surround},
    style::{
        theme::{Palette, Theme},
        BackgroundColor, ColorPair, ForegroundColor, Style,
    },
    ui::{Border, ComponentBorder, ComponentPadding},
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

    pub fn theme(&self) -> Theme {
        Theme::new(
            self.font_family.clone(),
            Style::default().with(ForegroundColor(ColorPair {
                light_color: self.palette.light.control.text.normal(),
                dark_color: self.palette.dark.control.text.normal(),
            })),
        )
        .when(
            |c| c.id.eq("root"),
            |style| {
                style.with(BackgroundColor(ColorPair {
                    light_color: self.palette.light.default.background.normal(),
                    dark_color: self.palette.dark.default.background.normal(),
                }))
            },
        )
        .when(
            |c| c.classes.contains("control"),
            |style| style.with(ComponentPadding(Surround::uniform(Points::new(10.)))),
        )
        .when(
            |c| {
                c.classes
                    .contains("control")
                    .and(!c.classes.contains("clear-background"))
            },
            |style| {
                style.with(BackgroundColor(ColorPair {
                    light_color: self.palette.light.control.background.normal(),
                    dark_color: self.palette.dark.control.background.normal(),
                }))
            },
        )
        .when(
            |c| {
                c.classes
                    .contains("control")
                    .and(c.is_active())
                    .and(!c.classes.contains("clear-background"))
            },
            |style| {
                style.with(BackgroundColor(ColorPair {
                    light_color: self.palette.light.control.background.darker(),
                    dark_color: self.palette.dark.control.background.darker(),
                }))
            },
        )
        .when(
            |c| {
                c.classes
                    .contains("control")
                    .and(c.is_hovered())
                    .and(!c.classes.contains("clear-background"))
            },
            |style| {
                style.with(BackgroundColor(ColorPair {
                    light_color: self.palette.light.control.background.lighter(),
                    dark_color: self.palette.dark.control.background.lighter(),
                }))
            },
        )
        .when(
            |c| {
                c.classes
                    .contains("control")
                    .and(c.classes.contains("text"))
            },
            |style| {
                style.with(ComponentBorder::uniform(Border::new(
                    2.,
                    ColorPair {
                        light_color: self.palette.light.control.background.darker(),
                        dark_color: self.palette.dark.control.background.lighter(),
                    },
                )))
            },
        )
        .when(
            |c| {
                c.classes
                    .contains("control")
                    .and(c.classes.contains("text"))
                    .and(c.is_focused())
            },
            |style| {
                style.with(ComponentBorder::uniform(Border::new(
                    2.,
                    self.palette.primary.normal().into(),
                )))
            },
        )
        .when(
            |c| {
                c.classes
                    .contains("control")
                    .and(c.classes.contains("text"))
                    .and(c.is_active())
            },
            |style| {
                style.with(BackgroundColor(ColorPair {
                    light_color: self.palette.light.control.background.normal(),
                    dark_color: self.palette.dark.control.background.normal(),
                }))
            },
        )
    }
}

impl Default for Minimal {
    fn default() -> Self {
        Self::new("Roboto", Default::default())
    }
}
