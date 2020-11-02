use crate::{
    math::{Raw, Scaled, Surround},
    style::{
        BackgroundColor, ColorPair, FallbackStyle, GenericStyle, Style, StyleComponent,
        UnscaledFallbackStyle, UnscaledStyleComponent,
    },
    ui::{component::Component, Layout, StyledContext},
    KludgineResult,
};
use async_trait::async_trait;
use euclid::Scale;

use super::{
    control::{ComponentBorder, ControlBorder, ControlPadding},
    StandaloneComponent,
};

#[derive(Debug, Clone, Default)]
pub struct PanePadding<Unit>(pub Surround<f32, Unit>);

impl StyleComponent<Scaled> for PanePadding<Scaled> {
    fn scale(&self, scale: Scale<f32, Scaled, Raw>, destination: &mut Style<Raw>) {
        destination.push(PanePadding(self.0 * scale))
    }
}

impl StyleComponent<Raw> for PanePadding<Raw> {
    fn scale(&self, _scale: Scale<f32, Raw, Raw>, map: &mut Style<Raw>) {
        map.push(PanePadding(self.0));
    }
}

impl FallbackStyle<Scaled> for PanePadding<Scaled> {
    fn lookup(style: &Style<Scaled>) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlPadding::<Scaled>::lookup(style).map(|cp| PanePadding(cp.0)))
    }
}

impl FallbackStyle<Raw> for PanePadding<Raw> {
    fn lookup(style: &Style<Raw>) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlPadding::<Raw>::lookup(style).map(|cp| PanePadding(cp.0)))
    }
}

#[derive(Debug, Default)]
pub struct Pane {}

#[async_trait]
impl Component for Pane {
    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        self.render_standard_background::<PaneBackgroundColor, PaneBorder>(context, layout)
            .await
    }
}

#[async_trait]
impl StandaloneComponent for Pane {}

#[derive(Debug, Clone)]
pub struct PaneBackgroundColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for PaneBackgroundColor {
    fn unscaled_should_be_inherited(&self) -> bool {
        false
    }
}

impl Default for PaneBackgroundColor {
    fn default() -> Self {
        Self(BackgroundColor::default().0)
    }
}

impl UnscaledFallbackStyle for PaneBackgroundColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| BackgroundColor::lookup_unscaled(style).map(|fg| PaneBackgroundColor(fg.0)))
    }
}

impl Into<ColorPair> for PaneBackgroundColor {
    fn into(self) -> ColorPair {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct PaneBorder(pub ComponentBorder);
impl UnscaledStyleComponent<Scaled> for PaneBorder {}

impl UnscaledFallbackStyle for PaneBorder {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlBorder::lookup_unscaled(style).map(|cb| PaneBorder(cb.0)))
    }
}

impl Into<ComponentBorder> for PaneBorder {
    fn into(self) -> ComponentBorder {
        self.0
    }
}
