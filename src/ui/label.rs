use crate::{
    event::MouseButton,
    math::{Point, Points, Scaled, Size},
    style::{Alignment, EffectiveStyle},
    text::{wrap::TextWrap, Text},
    ui::{
        Component, Context, ControlEvent, InteractiveComponent, Layout, SceneContext, StyledContext,
    },
    KludgineResult,
};
use async_trait::async_trait;

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
    type Input = LabelCommand;
    type Message = ();
    type Output = ControlEvent;

    async fn receive_input(
        &mut self,
        context: &mut Context,
        message: Self::Input,
    ) -> KludgineResult<()> {
        match message {
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
    async fn update(&mut self, _context: &mut SceneContext) -> KludgineResult<()> {
        Ok(())
    }

    async fn render(&self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        let text = self.create_text(context.effective_style());
        text.render_at(
            context.scene(),
            layout.inner_bounds().origin,
            self.wrapping(
                &layout.inner_bounds().size,
                context.effective_style().alignment,
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
            context.effective_style().alignment,
        );
        let wrapped_size = text.wrap(context.scene(), wrapping).await?.size().await;
        Ok(wrapped_size / context.scene().effective_scale_factor().await)
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
}

impl Label {
    pub fn new(value: impl ToString) -> Self {
        Self {
            value: value.to_string(),
        }
    }
    fn create_text(&self, effective_style: &EffectiveStyle) -> Text {
        Text::span(&self.value, effective_style)
    }

    fn wrapping(&self, size: &Size<f32, Scaled>, alignment: Alignment) -> TextWrap {
        TextWrap::SingleLine {
            max_width: Points::new(size.width),
            truncate: true,
            alignment,
        }
    }
}
