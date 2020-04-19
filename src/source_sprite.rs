use super::{math::Rect, texture::Texture, KludgineHandle};
#[derive(Clone)]
pub struct SourceSprite {
    pub(crate) handle: KludgineHandle<SourceSpriteData>,
}

pub(crate) struct SourceSpriteData {
    pub location: Rect<u32>,
    pub texture: Texture,
}

impl SourceSprite {
    pub fn new(location: Rect<u32>, texture: &Texture) -> Self {
        SourceSprite {
            handle: KludgineHandle::new(SourceSpriteData {
                location,
                texture: texture.clone(),
            }),
        }
    }

    pub fn entire_texture(texture: &Texture) -> Self {
        let (w, h) = {
            let texture = texture.handle.read().expect("Error reading source sprice");
            (texture.image.width(), texture.image.height())
        };
        Self::new(Rect::sized(0, 0, w, h), &texture)
    }
}
