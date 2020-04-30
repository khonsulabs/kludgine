use crate::{
    math::{Point, Rect, Size},
    scene::SceneTarget,
    text::{wrap::TextWrap, Text},
    ui::view::{BaseView, View, ViewCore},
    KludgineResult,
};
use async_trait::async_trait;
use kludgine_macros::ViewCore;

#[derive(ViewCore, Debug, Default, Clone)]
pub struct Label {
    view: BaseView,
    value: Option<String>,
}

#[async_trait]
impl View for Label {
    async fn render<'a>(&self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        let font = scene
            .lookup_font(
                &self.view.effective_style.font_family,
                self.view.effective_style.font_weight,
            )
            .await?;
        let metrics = font.metrics(self.view.effective_style.font_size).await;
        match self.create_text()? {
            Some(text) => {
                text.render_at(
                    scene,
                    Point::new(
                        self.view.bounds.origin.x,
                        self.view.bounds.origin.y + metrics.ascent / scene.effective_scale_factor(),
                    ),
                    self.wrapping(&self.view.bounds.size),
                )
                .await
            }
            None => Ok(()),
        }
    }

    async fn layout_within<'a>(
        &mut self,
        scene: &mut SceneTarget<'a>,
        bounds: Rect,
    ) -> KludgineResult<()> {
        self.view
            .layout_within(&self.content_size(&bounds.size, scene).await?, bounds)
    }

    async fn content_size<'a>(
        &self,
        maximum_size: &Size,
        scene: &mut SceneTarget<'a>,
    ) -> KludgineResult<Size> {
        let size = match self.create_text()? {
            Some(text) => {
                text.wrap(
                    scene,
                    self.wrapping(&self.view.layout.size_with_minimal_padding(&maximum_size)),
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

    fn create_text(&self) -> KludgineResult<Option<Text>> {
        if let Some(value) = &self.value {
            Ok(Some(Text::span(value, &self.view.effective_style)))
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
