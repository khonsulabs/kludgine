use super::{
    math::Rect,
    texture::{LoadedTexture, Texture},
    KludgineHandle,
};
#[derive(Clone)]
pub struct SourceSprite {
    pub(crate) handle: KludgineHandle<SourceSpriteData>,
}

pub(crate) struct SourceSpriteData {
    pub location: Rect,
    pub texture: Texture,
}

impl SourceSprite {
    pub fn new(location: Rect, texture: Texture) -> Self {
        SourceSprite {
            handle: KludgineHandle::new(SourceSpriteData { location, texture }),
        }
    }

    pub fn entire_texture(texture: Texture) -> Self {
        let (w, h) = {
            let texture = texture.handle.read().expect("Error reading source sprice");
            (texture.image.width() as f32, texture.image.height() as f32)
        };
        Self::new(Rect::sized(0.0, 0.0, w, h), texture)
    }
}

#[derive(Clone)]
pub struct Sprite {
    pub(crate) handle: KludgineHandle<SpriteData>,
}

impl Sprite {
    pub fn new(render_at: Rect, source: SourceSprite) -> Self {
        Self {
            handle: KludgineHandle::new(SpriteData { render_at, source }),
        }
    }
}

pub(crate) struct SpriteData {
    pub render_at: Rect,
    pub source: SourceSprite,
}

pub(crate) struct SpriteBatch {
    pub loaded_texture: LoadedTexture,
    pub sprites: Vec<Sprite>,
}

impl SpriteBatch {
    pub fn new(loaded_texture: LoadedTexture) -> Self {
        SpriteBatch {
            loaded_texture,
            sprites: Vec::new(),
        }
    }
}
