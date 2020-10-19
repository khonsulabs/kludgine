use crate::{
    color::Color,
    event::MouseButton,
    math::{Point, Scaled},
    style::TextColor,
    style::{BackgroundColor, GenericStyle, UnscaledFallbackStyle, UnscaledStyleComponent},
};

#[derive(Clone, Debug)]
pub enum ControlEvent {
    Clicked {
        button: MouseButton,
        window_position: Point<f32, Scaled>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ControlBackgroundColor(pub Color);
impl UnscaledStyleComponent<Scaled> for ControlBackgroundColor {}

impl UnscaledFallbackStyle for ControlBackgroundColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style.get::<Self>().cloned().or_else(|| {
            BackgroundColor::lookup_unscaled(style).map(|fg| ControlBackgroundColor(fg.0))
        })
    }
}

impl Into<Color> for ControlBackgroundColor {
    fn into(self) -> Color {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct ControlTextColor(pub Color);
impl UnscaledStyleComponent<Scaled> for ControlTextColor {}

impl UnscaledFallbackStyle for ControlTextColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| TextColor::lookup_unscaled(style).map(|fg| ControlTextColor(fg.0)))
    }
}

impl Into<Color> for ControlTextColor {
    fn into(self) -> Color {
        self.0
    }
}
