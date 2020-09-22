use crate::{
    event::MouseButton,
    math::{Point, Points, Scaled, Size, Surround},
    style::{Style, StyleSheet},
    ui::{
        AbsoluteBounds, Component, Context, ControlEvent, Entity, InteractiveComponent, Label,
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
        let control_colors = theme.light_control();
        let mut style_sheet = context.style_sheet().await;

        self.label = self
            .new_entity(context, Label::new(&self.caption))
            .style_sheet(StyleSheet::from(Style {
                color: Some(control_colors.text.normal()),
                ..Default::default()
            }))
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

        style_sheet.normal.background_color = style_sheet
            .normal
            .background_color
            .or_else(|| Some(control_colors.background.normal()));

        style_sheet.hover.background_color = style_sheet
            .hover
            .background_color
            .or_else(|| Some(control_colors.background.lighter()));

        style_sheet.active.background_color = style_sheet
            .active
            .background_color
            .or_else(|| Some(control_colors.background.darker()));

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
