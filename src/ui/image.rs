use crate::{
    math::{Rect, Size},
    source_sprite::SourceSprite,
    sprite::Sprite,
    ui::{Component, Placements, SceneContext, StyledContext},
    KludgineResult,
};
use async_trait::async_trait;

#[derive(Debug)]
pub struct Image {
    sprite: Sprite,
    current_frame: Option<SourceSprite>,
}

#[async_trait]
impl Component for Image {
    type Message = ();

    async fn update(&mut self, context: &mut SceneContext) -> KludgineResult<()> {
        self.current_frame = Some(
            self.sprite
                .get_frame(context.scene().elapsed().await)
                .await?,
        );
        Ok(())
    }

    async fn render(&self, context: &mut StyledContext, location: &Rect) -> KludgineResult<()> {
        if let Some(frame) = &self.current_frame {
            frame.render_at(context.scene(), location.origin).await
        }
        Ok(())
    }

    async fn layout_within(
        &self,
        _context: &mut StyledContext,
        _max_size: &Size,
        _placements: &Placements,
    ) -> KludgineResult<Size> {
        if let Some(frame) = &self.current_frame {
            Ok(frame.size().await.into())
        } else {
            Ok(Size::default())
        }
    }
}

impl Image {
    pub fn new(sprite: Sprite) -> Self {
        Self {
            sprite,
            current_frame: None,
        }
    }
}
