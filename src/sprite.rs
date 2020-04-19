use super::{math::Rect, source_sprite::SourceSprite, texture::LoadedTexture, KludgineHandle};

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
