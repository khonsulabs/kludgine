use crate::{
    math::{Point, Size},
    scene::SceneTarget,
    style::EffectiveStyle,
    text::{wrap::TextWrap, Text},
    ui::{Component, Controller},
    KludgineResult,
};
use async_trait::async_trait;

#[derive(Debug, Default, Clone)]
pub struct Label {
    value: Option<String>,
}

#[async_trait]
impl Controller for Label {
    async fn render(
        &self,
        component: &Component,
        scene: &mut SceneTarget<'_>,
    ) -> KludgineResult<()> {
        let effective_style = component.effective_style().await;
        let bounds = component.bounds().await;
        let font = scene
            .lookup_font(&effective_style.font_family, effective_style.font_weight)
            .await?;
        let metrics = font.metrics(effective_style.font_size).await;
        match self.create_text(&effective_style)? {
            Some(text) => {
                text.render_at(
                    scene,
                    Point::new(
                        bounds.origin.x,
                        bounds.origin.y + metrics.ascent / scene.effective_scale_factor(),
                    ),
                    self.wrapping(&bounds.size),
                )
                .await
            }
            None => Ok(()),
        }
    }

    async fn content_size(
        &self,
        component: &Component,
        maximum_size: &Size,
        scene: &mut SceneTarget<'_>,
    ) -> KludgineResult<Size> {
        let size = match self.create_text(&component.effective_style().await)? {
            Some(text) => {
                text.wrap(
                    scene,
                    self.wrapping(
                        &component
                            .layout()
                            .await
                            .size_with_minimal_padding(&maximum_size),
                    ),
                )
                .await?
                .size()
                .await
                    / scene.effective_scale_factor()
            }
            None => Size::default(),
        };
        Ok(size)
    }
}

impl Label {
    pub fn with_value<S: Into<String>>(&mut self, value: S) -> &mut Self {
        self.value = Some(value.into());
        self
    }

    fn create_text(&self, effective_style: &EffectiveStyle) -> KludgineResult<Option<Text>> {
        if let Some(value) = &self.value {
            Ok(Some(Text::span(value, effective_style)))
        } else {
            Ok(None)
        }
    }

    fn wrapping(&self, size: &Size) -> TextWrap {
        TextWrap::SingleLine {
            max_width: size.width,
            truncate: true,
        }
    }
}
