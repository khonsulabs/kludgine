use crate::{
    math::{Rect, Size},
    scene::SceneTarget,
    source_sprite::SourceSprite,
    sprite::Sprite,
    ui::{Component, Context},
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

    async fn update(&mut self, _context: &mut Context, scene: &SceneTarget) -> KludgineResult<()> {
        self.current_frame = Some(self.sprite.get_frame(scene.elapsed().await).await?);
        Ok(())
    }

    async fn render(
        &self,
        _context: &mut Context,
        scene: &SceneTarget,
        location: Rect,
    ) -> KludgineResult<()> {
        if let Some(frame) = &self.current_frame {
            frame.render_at(scene, location.origin).await
        }
        Ok(())
    }

    async fn content_size(&self, _context: &mut Context, _max_size: Size) -> KludgineResult<Size> {
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
