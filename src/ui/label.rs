use crate::{
    math::{Point, Rect, Size},
    style::EffectiveStyle,
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

    async fn render(&self, context: &mut StyledContext, bounds: Rect) -> KludgineResult<()> {
        let font = context
            .scene()
            .lookup_font(
                &context.effective_style().font_family,
                context.effective_style().font_weight,
            )
            .await?;
        let metrics = font.metrics(context.effective_style().font_size).await;
        let text = self.create_text(context.effective_style());
        text.render_at(
            context.scene(),
            Point::new(
                bounds.origin.x,
                bounds.origin.y + metrics.ascent / context.scene().effective_scale_factor().await,
            ),
            self.wrapping(&bounds.size),
        )
        .await
    }

    async fn layout_within(
        &self,
        context: &mut StyledContext,
        max_size: Size,
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
    pub fn new(value: String) -> Self {
        Self { value }
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
