use crate::{
    math::{Raw, Rect, Size},
    sprite::RenderedSprite,
};

pub(crate) struct Batch {
    pub size: Size<u32>,
    pub clipping_rect: Option<Rect<u32, Raw>>,
    pub loaded_texture_id: u64,
    pub sprites: Vec<RenderedSprite>,
}

impl Batch {
    pub fn new(
        loaded_texture_id: u64,
        size: Size<u32>,
        clipping_rect: Option<Rect<u32, Raw>>,
    ) -> Self {
        Self {
            loaded_texture_id,
            size,
            clipping_rect,
            sprites: Vec::new(),
        }
    }
}
