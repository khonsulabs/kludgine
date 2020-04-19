use super::{math::Rect, source_sprite::SourceSprite, texture::Texture};
use crossbeam::atomic::AtomicCell;
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref GLOBAL_ID_CELL: AtomicCell<usize> = { AtomicCell::new(0) };
}

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
pub struct AtlasId(usize);

impl AtlasId {
    pub(crate) fn new() -> Self {
        AtlasId(GLOBAL_ID_CELL.fetch_add(1))
    }
}

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
pub struct AtlasSpriteId(AtlasId, usize);

pub struct Atlas {
    pub(crate) id: AtlasId,
    pub(crate) texture: Texture,
    pub(crate) sprites: HashMap<AtlasSpriteId, SourceSprite>,
}

impl From<Texture> for Atlas {
    fn from(texture: Texture) -> Self {
        Atlas {
            id: AtlasId::new(),
            texture,
            sprites: HashMap::new(),
        }
    }
}

impl Atlas {
    pub fn define_sprite(&mut self, rect: Rect<u32>) -> AtlasSpriteId {
        let next_id = AtlasSpriteId(self.id, self.sprites.len());

        self.sprites
            .insert(next_id, SourceSprite::new(rect, &self.texture));

        next_id
    }

    pub fn get(&self, id: &AtlasSpriteId) -> Option<SourceSprite> {
        if let Some(sprite) = self.sprites.get(id) {
            Some(sprite.clone())
        } else {
            None
        }
    }
}
