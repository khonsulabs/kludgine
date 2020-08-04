use super::{
    math::{Pixels, Point, Points, Rect, ScreenMeasurement, Size},
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
        Self::new(Rect::sized(Point::default(), Size::new(w, h)), texture)
    }

    pub async fn render_at(&self, scene: &SceneTarget, location: Point<Points>) {
        self.render_at_with_alpha(scene, location, 1.).await
    }

    pub async fn render_within(&self, scene: &SceneTarget, bounds: Rect<Points>) {
        self.render_with_alpha(scene, bounds, 1.).await
    }

    pub async fn render_at_with_alpha(
        &self,
        scene: &SceneTarget,
        location: Point<Points>,
        alpha: f32,
    ) {
        let sprite_location = self.location().await;
        self.render_with_alpha(
            scene,
            Rect::sized(
                location,
                Size::new(
                    Points::from_f32(sprite_location.size.width as f32),
                    Points::from_f32(sprite_location.size.height as f32),
                ),
            ),
            alpha,
        )
        .await
    }

    pub async fn render_with_alpha(&self, scene: &SceneTarget, bounds: Rect<Points>, alpha: f32) {
        let translated_origin = scene
            .user_to_device_point(Point::new(
                bounds.origin.x,
                bounds.origin.y + bounds.size.height,
            ))
            .await;
        let destination = Rect::sized(translated_origin, bounds.size)
            .to_pixels(scene.effective_scale_factor().await);
        scene
            .push_element(Element::Sprite(RenderedSprite::new(
                destination.to_f32(),
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
