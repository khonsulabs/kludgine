use crate::{
    math::{Points, Surround},
    style::{
        theme::{Palette, Theme},
        BackgroundColor, ColorPair, ForegroundColor, Style,
    },
    ui::{
        Border, ComponentBorder, ComponentPadding, DialogButtonSpacing, ScrollbarGripColor,
        ScrollbarSize,
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

    pub fn theme(&self) -> Theme {
        Theme::new(
            self.font_family.clone(),
            Style::default().with(ForegroundColor(ColorPair {
                light_color: self.palette.light.control.text.normal(),
                dark_color: self.palette.dark.control.text.normal(),
            })),
        )
        // The root component draws a solid background
        .when(
            |c| c.id.eq("root"),
            |style| {
                style.with(BackgroundColor(ColorPair {
                    light_color: self.palette.light.default.background.normal(),
                    dark_color: self.palette.dark.default.background.normal(),
                }))
            },
        )
        // All controls have padding built into them
        .when(
            |c| c.classes.contains("control"),
            |style| style.with(ComponentPadding(Surround::uniform(Points::new(10.)))),
        )
        // All controls that don't have a "clear-background" class will have a background color
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
        // All controls that don't have a "clear-background" class will have a background color when active
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
        // All controls that don't have a "clear-background" class will have a background color when hovered
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
        // "is-primary"
        .when(
            |c| c.classes.contains("is-primary"),
            |style| {
                style.with(BackgroundColor(ColorPair {
                    light_color: self.palette.primary.normal(),
                    dark_color: self.palette.primary.normal(),
                }))
            },
        )
        .when(
            |c| c.classes.contains("is-primary").and(c.is_hovered()),
            |style| {
                style.with(BackgroundColor(ColorPair {
                    light_color: self.palette.primary.lighter(),
                    dark_color: self.palette.primary.lighter(),
                }))
            },
        )
        .when(
            |c| c.classes.contains("is-primary").and(c.is_active()),
            |style| {
                style.with(BackgroundColor(ColorPair {
                    light_color: self.palette.primary.darker(),
                    dark_color: self.palette.primary.darker(),
                }))
            },
        )
        // Text input
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
        // Toast
        .when(
            |c| c.classes.contains("toast"),
            |style| {
                style
                    .with(ComponentPadding(Surround::uniform(Points::new(10.))))
                    .with(ComponentBorder::uniform(Border::new(
                        2.,
                        ColorPair {
                            light_color: self.palette.light.control.background.darker(),
                            dark_color: self.palette.dark.control.background.lighter(),
                        },
                    )))
                    .with(BackgroundColor(ColorPair {
                        light_color: self.palette.light.control.background.normal(),
                        dark_color: self.palette.dark.control.background.normal(),
                    }))
            },
        )
        // Dialog
        .when(
            |c| c.classes.contains("dialog"),
            |style| {
                style
                    .with(DialogButtonSpacing(Points::new(10.)))
                    .with(ComponentPadding(Surround::uniform(Points::new(10.))))
                    .with(ComponentBorder::uniform(Border::new(
                        2.,
                        ColorPair {
                            light_color: self.palette.light.control.background.darker(),
                            dark_color: self.palette.dark.control.background.lighter(),
                        },
                    )))
                    .with(BackgroundColor(ColorPair {
                        light_color: self.palette.light.control.background.normal(),
                        dark_color: self.palette.dark.control.background.normal(),
                    }))
            },
        )
        // Scrollbar
        .when(
            |c| c.classes.contains("scrollbar"),
            |style| {
                style
                    .with(ScrollbarSize(Points::new(10.)))
                    .with(ScrollbarGripColor(ColorPair {
                        light_color: self.palette.primary.normal(),
                        dark_color: self.palette.primary.normal(),
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
