use std::fmt::Debug;

use crate::{
    color::Color,
    event::MouseButton,
    math::{Point, Raw, Scale, Scaled, Surround},
    style::{
        BackgroundColor, FallbackStyle, GenericStyle, Style, StyleComponent, TextColor,
        UnscaledFallbackStyle, UnscaledStyleComponent,
    },
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

#[derive(Debug, Clone, Default)]
pub struct ControlPadding<Unit>(pub Surround<f32, Unit>);

impl StyleComponent<Scaled> for ControlPadding<Scaled> {
    fn scale(&self, scale: Scale<f32, Scaled, Raw>, destination: &mut Style<Raw>) {
        destination.push(ControlPadding(self.0 * scale))
    }
}

impl StyleComponent<Raw> for ControlPadding<Raw> {
    fn scale(&self, _scale: Scale<f32, Raw, Raw>, map: &mut Style<Raw>) {
        map.push(ControlPadding(self.0));
    }
}

impl FallbackStyle<Scaled> for ControlPadding<Scaled> {}
impl FallbackStyle<Raw> for ControlPadding<Raw> {}
