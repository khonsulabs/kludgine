use crate::{
    event::MouseButton,
    math::{Point, Points, Raw, Scaled, Size},
    style::{Alignment, Style},
    text::{wrap::TextWrap, Text},
    ui::{
        Component, Context, ControlBackgroundColor, ControlEvent, InteractiveComponent, Layout,
        StyledContext,
    },
    KludgineResult,
};
use async_trait::async_trait;

use super::control::ControlBorder;

#[derive(Debug)]
pub struct Label {
    value: String,
}

#[derive(Clone, Debug)]
pub enum LabelCommand {
    SetValue(String),
}

#[async_trait]
impl InteractiveComponent for Label {
    type Command = LabelCommand;
    type Message = ();
    type Event = ControlEvent;

    async fn receive_command(
        &mut self,
        context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        match command {
            LabelCommand::SetValue(new_value) => {
                if self.value != new_value {
                    self.value = new_value;
                    context.set_needs_redraw().await;
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Component for Label {
    async fn update(&mut self, _context: &mut Context) -> KludgineResult<()> {
        Ok(())
    }

    async fn render(&mut self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        let text = self.create_text(context.effective_style());
        text.render_at(
            context.scene(),
            layout.inner_bounds().origin,
            self.wrapping(
                &layout.inner_bounds().size,
                context.effective_style().get_or_default::<Alignment>(),
            ),
        )
        .await
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let text = self.create_text(context.effective_style());
        let wrapping = self.wrapping(
            &Size::new(
                constraints.width.unwrap_or_else(|| f32::MAX),
                constraints.height.unwrap_or_else(|| f32::MAX),
            ),
            context.effective_style().get_or_default::<Alignment>(),
        );
        let wrapped_size = text.wrap(context.scene(), wrapping).await?.size().await;
        Ok(wrapped_size / context.scene().scale_factor().await)
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

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        self.render_standard_background::<ControlBackgroundColor, ControlBorder>(context, layout)
            .await
    }
}

impl Label {
    pub fn new(value: impl ToString) -> Self {
        Self {
            value: value.to_string(),
        }
    }
    fn create_text(&self, effective_style: &Style<Raw>) -> Text {
        Text::span(&self.value, effective_style.clone())
    }

    fn wrapping(&self, size: &Size<f32, Scaled>, alignment: Alignment) -> TextWrap {
        TextWrap::SingleLine {
            max_width: Points::new(size.width),
            truncate: true,
            alignment,
        }
    }
}
