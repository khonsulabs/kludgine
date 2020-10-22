use std::fmt::Debug;

use crate::{
    event::MouseButton,
    math::{Point, Raw, Scale, Scaled, Surround},
    style::{
        BackgroundColor, ColorPair, FallbackStyle, GenericStyle, Style, StyleComponent, TextColor,
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

#[derive(Debug, Clone)]
pub struct ControlBackgroundColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for ControlBackgroundColor {
    fn unscaled_should_be_inherited(&self) -> bool {
        false
    }
}

impl Default for ControlBackgroundColor {
    fn default() -> Self {
        Self(BackgroundColor::default().0)
    }
}

impl UnscaledFallbackStyle for ControlBackgroundColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style.get::<Self>().cloned().or_else(|| {
            BackgroundColor::lookup_unscaled(style).map(|fg| ControlBackgroundColor(fg.0))
        })
    }
}

impl Into<ColorPair> for ControlBackgroundColor {
    fn into(self) -> ColorPair {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct ControlTextColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for ControlTextColor {}

impl Default for ControlTextColor {
    fn default() -> Self {
        Self(TextColor::default().0)
    }
}

impl UnscaledFallbackStyle for ControlTextColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| TextColor::lookup_unscaled(style).map(|fg| ControlTextColor(fg.0)))
    }
}

impl Into<ColorPair> for ControlTextColor {
    fn into(self) -> ColorPair {
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
