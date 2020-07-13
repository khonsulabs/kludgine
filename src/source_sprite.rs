use super::{
    math::Point,
    math::{Rect, Size},
    scene::{Element, SceneTarget},
    sprite::RenderedSprite,
    texture::Texture,
    KludgineHandle,
};
#[derive(Debug, Clone)]
pub struct SourceSprite {
    pub(crate) handle: KludgineHandle<SourceSpriteData>,
}

#[derive(Debug)]
pub(crate) struct SourceSpriteData {
    pub location: Rect<u32>,
    pub texture: Texture,
}

impl SourceSprite {
    pub fn new(location: Rect<u32>, texture: Texture) -> Self {
        SourceSprite {
            handle: KludgineHandle::new(SourceSpriteData { location, texture }),
        }
    }

    pub async fn entire_texture(texture: Texture) -> Self {
        let (w, h) = {
            let texture = texture.handle.read().await;
            (texture.image.width(), texture.image.height())
        };
        Self::new(Rect::sized(Point::new(0, 0), Size::new(w, h)), texture)
    }

    pub async fn render_at(&self, scene: &SceneTarget, location: Point) {
        let (w, h) = {
            let source = self.handle.read().await;
            (
                source.location.size.width as f32,
                source.location.size.height as f32,
            )
        };
        let location = scene
            .user_to_device_point(Point::new(location.x, location.y + h))
            .await;
        let effective_scale_factor = scene.effective_scale_factor().await;
        scene.push_element(Element::Sprite(RenderedSprite::new(
            Rect::sized(
                Point::new(
                    location.x * effective_scale_factor,
                    location.y * effective_scale_factor,
                ),
                Size::new(w * effective_scale_factor, h * effective_scale_factor),
            ),
            self.clone(),
        ))).await;
    }

    pub async fn size(&self) -> Size<u32> {
        let sprite = self.handle.read().await;
        sprite.texture.size().await
    }
}
