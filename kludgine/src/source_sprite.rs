use super::{
    math::Point,
    math::{Rect, Size},
    scene::{Element, SceneTarget},
    sprite::RenderedSprite,
    texture::Texture,
    KludgineHandle,
};
#[derive(Clone)]
pub struct SourceSprite {
    pub(crate) handle: KludgineHandle<SourceSpriteData>,
}

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

    pub fn entire_texture(texture: Texture) -> Self {
        let (w, h) = {
            let texture = texture.handle.read().expect("Error reading source sprice");
            (texture.image.width(), texture.image.height())
        };
        Self::new(Rect::sized(Point::new(0, 0), Size::new(w, h)), texture)
    }

    pub fn render_at(&self, scene: &mut SceneTarget, location: Point) {
        let (w, h) = {
            let source = self.handle.read().expect("Error locking source_sprite");
            (
                source.location.size.width as f32,
                source.location.size.height as f32,
            )
        };
        let location = scene.user_to_device_point(Point::new(location.x, location.y + h));
        let effective_scale_factor = scene.effective_scale_factor();
        scene.push_element(Element::Sprite(RenderedSprite::new(
            Rect::sized(
                Point::new(
                    location.x * effective_scale_factor,
                    location.y * effective_scale_factor,
                ),
                Size::new(w * effective_scale_factor, h * effective_scale_factor),
            ),
            self.clone(),
        )));
    }
}
