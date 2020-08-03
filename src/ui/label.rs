use crate::{
    event::MouseButton,
    math::{Point, Points, Size},
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
        _context: &mut Context,
        message: Self::Input,
    ) -> KludgineResult<()> {
        match message {
            LabelCommand::SetValue(new_value) => {
                self.value = new_value;
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
            Point::new(
                layout.inner_bounds().origin.x,
                layout.inner_bounds().origin.y,
            ),
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
        constraints: &Size<Option<Points>>,
    ) -> KludgineResult<Size<Points>> {
        let text = self.create_text(context.effective_style());
        let wrapping = self.wrapping(
            &Size {
                width: constraints.width.unwrap_or(Points(f32::MAX)),
                height: constraints.height.unwrap_or(Points(f32::MAX)),
            },
            context.effective_style().alignment,
        );
        let wrapped_size = text.wrap(context.scene(), wrapping).await?.size().await;
        Ok(wrapped_size.to_points(context.scene().effective_scale_factor().await))
    }

    async fn clicked(
        &mut self,
        context: &mut Context,
        _window_position: &Point<Points>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        self.callback(context, ControlEvent::Clicked(button)).await;
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

    fn wrapping(&self, size: &Size<Points>, alignment: Alignment) -> TextWrap {
        TextWrap::SingleLine {
            max_width: size.width,
            truncate: true,
            alignment,
        }
    }
}
