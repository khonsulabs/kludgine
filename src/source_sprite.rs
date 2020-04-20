use super::{
    math::Point,
    math::Rect,
    scene::{Element, Scene},
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
        Self::new(Rect::sized(0, 0, w, h), texture)
    }

    pub fn render_at(&self, scene: &mut Scene, location: Point) {
        let (w, h) = {
            let source = self.handle.read().expect("Error locking source_sprite");
            (
                source.location.width() as f32,
                source.location.height() as f32,
            )
        };
        let location = scene.user_to_device_point(Point::new(location.x, location.y + h));
        scene.elements.push(Element::Sprite(RenderedSprite::new(
            Rect::sized(
                location.x * scene.scale_factor,
                location.y * scene.scale_factor,
                w * scene.scale_factor,
                h * scene.scale_factor,
            ),
            self.clone(),
        )));
    }
}
