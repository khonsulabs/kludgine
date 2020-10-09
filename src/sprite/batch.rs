use crate::{math::Size, sprite::RenderedSprite};

pub(crate) struct Batch {
    pub size: Size<u32>,
    pub loaded_texture_id: u64,
    pub sprites: Vec<RenderedSprite>,
}

impl Batch {
    pub fn new(loaded_texture_id: u64, size: Size<u32>) -> Self {
        Self {
            loaded_texture_id,
            size,
            sprites: Vec::new(),
        }
    }
}
