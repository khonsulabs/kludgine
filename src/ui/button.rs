use crate::{
    color::Color,
    event::MouseButton,
    math::{Point, Raw, Scaled, Size, Surround},
    style::{
        FallbackStyle, GenericStyle, Style, StyleComponent, StyleSheet, TextColor,
        UnscaledFallbackStyle, UnscaledStyleComponent,
    },
    ui::{
        component::{render_background, Component},
        control::{ControlBackgroundColor, ControlTextColor},
        AbsoluteBounds, Context, ControlEvent, Entity, InteractiveComponent, Label, Layout,
        StyledContext,
    },
    KludgineResult,
};
use async_trait::async_trait;
use euclid::Scale;

use super::control::ControlPadding;

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
        let style_sheet = context.style_sheet().await.inherit_from(&control_colors);

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
        render_background::<ButtonBackgroundColor>(context, layout).await
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

#[derive(Debug, Clone, Default)]
pub struct ButtonBackgroundColor(pub Color);
impl UnscaledStyleComponent<Scaled> for ButtonBackgroundColor {}

impl UnscaledFallbackStyle for ButtonBackgroundColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style.get::<Self>().cloned().or_else(|| {
            ControlBackgroundColor::lookup_unscaled(style).map(|fg| ButtonBackgroundColor(fg.0))
        })
    }
}

impl Into<Color> for ButtonBackgroundColor {
    fn into(self) -> Color {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct ButtonTextColor(pub Color);
impl UnscaledStyleComponent<Scaled> for ButtonTextColor {}

impl UnscaledFallbackStyle for ButtonTextColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlTextColor::lookup_unscaled(style).map(|fg| ButtonTextColor(fg.0)))
    }
}

impl Into<Color> for ButtonTextColor {
    fn into(self) -> Color {
        self.0
    }
}
