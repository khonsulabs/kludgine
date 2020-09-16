use crate::{
    math::{Point, Rect, Scaled, Size},
    scene::{Element, SceneTarget},
    sprite::{RenderedSprite, SpriteRotation},
    texture::Texture,
    KludgineHandle,
};
#[derive(Debug, Clone)]
pub struct SpriteSource {
    pub(crate) handle: KludgineHandle<SpriteSourceData>,
}

#[derive(Debug)]
pub(crate) struct SpriteSourceData {
    pub location: Rect<u32>,
    pub texture: Texture,
}

impl SpriteSource {
    pub fn new(location: Rect<u32>, texture: Texture) -> Self {
        SpriteSource {
            handle: KludgineHandle::new(SpriteSourceData { location, texture }),
        }
    }

    pub async fn entire_texture(texture: Texture) -> Self {
        let (w, h) = {
            let texture = texture.handle.read().await;
            (texture.image.width(), texture.image.height())
        };
        Self::new(Rect::new(Point::default(), Size::new(w, h)), texture)
    }

    pub async fn render_at(
        &self,
        scene: &SceneTarget,
        location: Point<f32, Scaled>,
        rotation: SpriteRotation<Scaled>,
    ) {
        self.render_at_with_alpha(scene, location, rotation, 1.)
            .await
    }

    pub async fn render_within(
        &self,
        scene: &SceneTarget,
        bounds: Rect<f32, Scaled>,
        rotation: SpriteRotation<Scaled>,
    ) {
        self.render_with_alpha(scene, bounds, rotation, 1.).await
    }

    pub async fn render_at_with_alpha(
        &self,
        scene: &SceneTarget,
        location: Point<f32, Scaled>,
        rotation: SpriteRotation<Scaled>,
        alpha: f32,
    ) {
        let sprite_location = self.location().await;
        self.render_with_alpha(
            scene,
            Rect::new(location, sprite_location.size.to_f32().cast_unit()),
            rotation,
            alpha,
        )
        .await
    }

    pub async fn render_with_alpha(
        &self,
        scene: &SceneTarget,
        bounds: Rect<f32, Scaled>,
        rotation: SpriteRotation<Scaled>,
        alpha: f32,
    ) {
        let effective_scale = scene.effective_scale_factor().await;
        let destination = bounds * effective_scale;
        scene
            .push_element(Element::Sprite(RenderedSprite::new(
                destination,
                rotation * effective_scale,
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
