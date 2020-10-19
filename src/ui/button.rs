use crate::{
    color::Color,
    event::MouseButton,
    math::{Point, Points, Scaled, Size, Surround},
    style::{
        FallbackStyle, GenericStyle, Style, StyleSheet, UnscaledFallbackStyle,
        UnscaledStyleComponent,
    },
    ui::{
        component::{render_background, Component},
        control::{ControlBackgroundColor, ControlTextColor},
        AbsoluteBounds, Context, ControlEvent, Entity, InteractiveComponent, Label, Layout,
        SceneContext, StyledContext,
    },
    KludgineResult,
};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    padding: Surround<f32, Scaled>,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            padding: Surround::uniform(Points::new(10.)),
        }
    }
}

#[derive(Debug)]
pub struct Button {
    caption: String,
    label: Entity<Label>,
    style: ButtonStyle,
}

#[async_trait]
impl Component for Button {
    async fn initialize(&mut self, context: &mut SceneContext) -> KludgineResult<()> {
        let theme = context.scene().theme().await;
        let control_colors = theme.default_style_sheet();
        let style_sheet = context.style_sheet().await.inherit_from(&control_colors);

        self.label = self
            .new_entity(context, Label::new(&self.caption))
            .style_sheet(StyleSheet::from(Style::new().with(
                ButtonTextColor::lookup(&style_sheet.normal).unwrap_or_default(),
            )))
            .bounds(AbsoluteBounds {
                left: crate::math::Dimension::from_f32(10.),
                top: crate::math::Dimension::from_f32(10.),
                right: crate::math::Dimension::from_f32(10.),
                bottom: crate::math::Dimension::from_f32(10.),
                ..Default::default()
            })
            .interactive(false)
            .insert()
            .await?;

        context.set_style_sheet(style_sheet).await;

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
        let contraints_minus_padding = self.style.padding.inset_constraints(constraints);
        Ok(context
            .content_size(&self.label, &contraints_minus_padding)
            .await?
            + self.style.padding.minimum_size())
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
            style: Default::default(),
        }
    }

    pub fn button_style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }
}

#[derive(Clone, Debug)]
pub enum ButtonCommand {
    SetCaption(String),
    SetButtonStyle(ButtonStyle),
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
            ButtonCommand::SetButtonStyle(style) => {
                self.style = style;
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
