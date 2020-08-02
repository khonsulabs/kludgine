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
        Self::new(Rect::sized(Point::new(0u32, 0), Size::new(w, h)), texture)
    }

    pub async fn render_at(&self, scene: &SceneTarget, location: Point) {
        self.render_at_with_alpha(scene, location, 1.).await
    }

    pub async fn render_within(&self, scene: &SceneTarget, bounds: Rect) {
        self.render_with_alpha(scene, bounds, 1.).await
    }

    pub async fn render_at_with_alpha(&self, scene: &SceneTarget, location: Point, alpha: f32) {
        let sprite_location = self.location().await;
        self.render_with_alpha(
            scene,
            Rect::sized(location, sprite_location.size.into()),
            alpha,
        )
        .await
    }

    pub async fn render_with_alpha(&self, scene: &SceneTarget, bounds: Rect, alpha: f32) {
        let translated_origin = scene.user_to_device_point(Point::new(bounds.origin.x, bounds.origin.y + bounds.size.height)).await;
        let scaled_bounds = bounds.size * scene.effective_scale_factor().await;
        let destination = Rect::sized(translated_origin, scaled_bounds);
        scene
            .push_element(Element::Sprite(RenderedSprite::new(
                destination,
                alpha,
                self.clone(),
            )))
            .await;
    }

    pub async fn location(&self) -> Rect<u32> {
        let sprite = self.handle.read().await;
        sprite.location
    }
}
