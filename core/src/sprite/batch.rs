use crate::{
    math::{Pixels, Rect, Size},
    sprite::RenderedSprite,
};

#[derive(Debug)]
pub struct Batch {
    pub size: Size<u32>,
    pub clipping_rect: Option<Rect<u32, Pixels>>,
    pub loaded_texture_id: u64,
    pub sprites: Vec<RenderedSprite>,
}

impl Batch {
    pub const fn new(
        loaded_texture_id: u64,
        size: Size<u32>,
        clipping_rect: Option<Rect<u32, Pixels>>,
    ) -> Self {
        Self {
            loaded_texture_id,
            size,
            clipping_rect,
            sprites: Vec::new(),
        }
    }
}
