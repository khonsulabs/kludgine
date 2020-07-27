use crate::{
    math::{Point, Size},
    style::EffectiveStyle,
    text::{wrap::TextWrap, Text},
    ui::{Component, Context, InteractiveComponent, Layout, SceneContext, StyledContext},
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
    type Output = ();

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
            self.wrapping(&layout.inner_bounds().size),
        )
        .await
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>>,
    ) -> KludgineResult<Size> {
        let text = self.create_text(context.effective_style());
        let wrapping = self.wrapping(&Size {
            width: constraints.width.unwrap_or(f32::MAX),
            height: constraints.height.unwrap_or(f32::MAX),
        });
        let wrapped_size = text.wrap(context.scene(), wrapping).await?.size().await;
        let size = wrapped_size / context.scene().effective_scale_factor().await;
        Ok(size)
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

    fn wrapping(&self, size: &Size) -> TextWrap {
        TextWrap::SingleLine {
            max_width: size.width,
            truncate: true,
        }
    }
}
