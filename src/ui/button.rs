use crate::{
    math::{Point, Raw, Scaled, Size, Surround},
    style::{
        ColorPair, FallbackStyle, GenericStyle, Style, StyleComponent, StyleSheet, TextColor,
        UnscaledFallbackStyle, UnscaledStyleComponent,
    },
    ui::{
        component::Component,
        control::{ControlBackgroundColor, ControlTextColor},
        AbsoluteBounds, Context, ControlEvent, Entity, InteractiveComponent, Label, Layout,
        StyledContext,
    },
    window::event::MouseButton,
    KludgineResult,
};
use async_trait::async_trait;
use euclid::Scale;

use super::control::{ComponentBorder, ControlBorder, ControlPadding};

#[derive(Debug, Clone, Default)]
pub struct ButtonPadding<Unit>(pub Surround<f32, Unit>);

impl StyleComponent<Scaled> for ButtonPadding<Scaled> {
    fn scale(&self, scale: Scale<f32, Scaled, Raw>, destination: &mut Style<Raw>) {
        destination.push(ButtonPadding(self.0 * scale))
    }
}

impl StyleComponent<Raw> for ButtonPadding<Raw> {
    fn scale(&self, _scale: Scale<f32, Raw, Raw>, map: &mut Style<Raw>) {
        map.push(ButtonPadding(self.0));
    }
}

impl FallbackStyle<Scaled> for ButtonPadding<Scaled> {
    fn lookup(style: &Style<Scaled>) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlPadding::<Scaled>::lookup(style).map(|cp| ButtonPadding(cp.0)))
    }
}

impl FallbackStyle<Raw> for ButtonPadding<Raw> {
    fn lookup(style: &Style<Raw>) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlPadding::<Raw>::lookup(style).map(|cp| ButtonPadding(cp.0)))
    }
}

#[derive(Debug)]
pub struct Button {
    caption: String,
    label: Entity<Label>,
}

#[async_trait]
impl Component for Button {
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        let theme = context.scene().theme().await;
        let control_colors = theme.default_style_sheet();
        let style_sheet = context
            .style_sheet()
            .await
            .merge_with(&control_colors, false);

        self.label = self
            .new_entity(context, Label::new(&self.caption))
            .style_sheet(StyleSheet::from(
                Style::new().with(TextColor(
                    ButtonTextColor::lookup(&style_sheet.normal)
                        .unwrap_or_default()
                        .into(),
                )),
            ))
            .bounds(AbsoluteBounds::from(
                ButtonPadding::<Scaled>::lookup(&style_sheet.normal)
                    .unwrap_or_default()
                    .0,
            ))
            .interactive(false)
            .insert()
            .await?;

        Ok(())
    }

    async fn clicked(
        &mut self,
        context: &mut Context,
        window_position: Point<f32, Scaled>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        self.callback(
            context,
            ControlEvent::Clicked {
                button,
                window_position,
            },
        )
        .await;
        Ok(())
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let padding = ButtonPadding::<Raw>::lookup(context.effective_style())
            .unwrap_or_default()
            .0
            / context.scene().scale_factor().await;

        let contraints_minus_padding = padding.inset_constraints(constraints);
        Ok(context
            .content_size(&self.label, &contraints_minus_padding)
            .await?
            + padding.minimum_size())
    }

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        self.render_standard_background::<ButtonBackgroundColor, ButtonBorder>(context, layout)
            .await
    }
}

impl Button {
    pub fn new(caption: impl ToString) -> Self {
        let caption = caption.to_string();
        Self {
            caption,
            label: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ButtonCommand {
    SetCaption(String),
}

#[async_trait]
impl InteractiveComponent for Button {
    type Event = ControlEvent;
    type Message = ();
    type Command = ButtonCommand;

    async fn receive_command(
        &mut self,
        _context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        match command {
            ButtonCommand::SetCaption(caption) => {
                self.caption = caption;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ButtonBackgroundColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for ButtonBackgroundColor {
    fn unscaled_should_be_inherited(&self) -> bool {
        false
    }
}

impl Default for ButtonBackgroundColor {
    fn default() -> Self {
        Self(ControlBackgroundColor::default().0)
    }
}

impl UnscaledFallbackStyle for ButtonBackgroundColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style.get::<Self>().cloned().or_else(|| {
            ControlBackgroundColor::lookup_unscaled(style).map(|fg| ButtonBackgroundColor(fg.0))
        })
    }
}

impl Into<ColorPair> for ButtonBackgroundColor {
    fn into(self) -> ColorPair {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct ButtonTextColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for ButtonTextColor {}

impl Default for ButtonTextColor {
    fn default() -> Self {
        Self(ControlTextColor::default().0)
    }
}

impl UnscaledFallbackStyle for ButtonTextColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlTextColor::lookup_unscaled(style).map(|fg| ButtonTextColor(fg.0)))
    }
}

impl Into<ColorPair> for ButtonTextColor {
    fn into(self) -> ColorPair {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct ButtonBorder(pub ComponentBorder);
impl UnscaledStyleComponent<Scaled> for ButtonBorder {}

impl UnscaledFallbackStyle for ButtonBorder {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlBorder::lookup_unscaled(style).map(|cb| ButtonBorder(cb.0)))
    }
}

impl Into<ComponentBorder> for ButtonBorder {
    fn into(self) -> ComponentBorder {
        self.0
    }
}
