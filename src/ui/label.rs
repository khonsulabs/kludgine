use crate::{
    math::{Point, Rect, Size},
    shape::{Fill, Shape},
    style::{Color, EffectiveStyle},
    text::{wrap::TextWrap, Text},
    ui::{Component, Placements, SceneContext, StyledContext},
    KludgineResult,
};
use async_trait::async_trait;

#[derive(Debug)]
pub struct Label {
    value: String,
}

#[async_trait]
impl Component for Label {
    type Message = ();

    async fn update(&mut self, _context: &mut SceneContext) -> KludgineResult<()> {
        Ok(())
    }

    async fn render(&self, context: &mut StyledContext, bounds: &Rect) -> KludgineResult<()> {
        let text = self.create_text(context.effective_style());
        context
            .scene()
            .draw_shape(Shape::rect(bounds.coord1(), bounds.coord2()).fill(Fill::Solid(Color::RED)))
            .await;
        text.render_at(
            context.scene(),
            Point::new(bounds.origin.x, bounds.origin.y),
            self.wrapping(&bounds.size),
        )
        .await
    }

    async fn layout_within(
        &self,
        context: &mut StyledContext,
        max_size: &Size,
        _placements: &Placements,
    ) -> KludgineResult<Size> {
        let text = self.create_text(context.effective_style());
        let wrapping = self.wrapping(&context.layout().await.size_with_minimal_padding(&max_size));
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
