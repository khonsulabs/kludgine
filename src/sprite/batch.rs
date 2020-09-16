use crate::{sprite::RenderedSprite, texture::LoadedTexture};

pub(crate) struct Batch {
    pub loaded_texture: LoadedTexture,
    pub sprites: Vec<RenderedSprite>,
}

impl Batch {
    pub fn new(loaded_texture: LoadedTexture) -> Self {
        Self {
            loaded_texture,
            sprites: Vec::new(),
        }
    }
}
