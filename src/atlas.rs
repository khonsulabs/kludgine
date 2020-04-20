use super::{
    math::{Rect, Size},
    source_sprite::SourceSprite,
    texture::Texture,
    KludgineError, KludgineHandle, KludgineResult,
};
use crossbeam::atomic::AtomicCell;
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref GLOBAL_ID_CELL: AtomicCell<u32> = { AtomicCell::new(0) };
}

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
pub struct AtlasId(u32);

impl AtlasId {
    pub(crate) fn new() -> Self {
        AtlasId(GLOBAL_ID_CELL.fetch_add(1))
    }
}

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
pub struct AtlasSpriteId(AtlasId, u32);

pub struct Atlas {
    handle: KludgineHandle<AtlasData>,
}

pub(crate) struct AtlasData {
    pub id: AtlasId,
    pub texture: Texture,
    pub sprites: HashMap<AtlasSpriteId, SourceSprite>,
}

impl From<Texture> for Atlas {
    fn from(texture: Texture) -> Self {
        Atlas {
            handle: KludgineHandle::new(AtlasData {
                id: AtlasId::new(),
                texture,
                sprites: HashMap::new(),
            }),
        }
    }
}

impl Atlas {
    pub fn define_sprite(&mut self, rect: Rect<u32>) -> AtlasSpriteId {
        let mut atlas = self
            .handle
            .write()
            .expect("Error locking atlas to define sprite");

        let next_id = AtlasSpriteId(atlas.id, atlas.sprites.len() as u32);
        let texture = atlas.texture.clone();

        atlas
            .sprites
            .insert(next_id, SourceSprite::new(rect, texture));

        next_id
    }

    pub fn get(&self, id: &AtlasSpriteId) -> Option<SourceSprite> {
        let atlas = self
            .handle
            .read()
            .expect("Error locking atlas to define sprite");
        if let Some(sprite) = atlas.sprites.get(id) {
            Some(sprite.clone())
        } else {
            None
        }
    }

    pub fn id(&self) -> AtlasId {
        let atlas = self
            .handle
            .read()
            .expect("Error locking atlas to retrieve id");
        atlas.id
    }

    pub fn size(&self) -> Size<u32> {
        let atlas = self
            .handle
            .read()
            .expect("Error locking atlas to retrieve id");
        atlas.texture.size()
    }
}

pub(crate) struct AtlasCollection {
    atlases: HashMap<AtlasId, Atlas>,
}

impl AtlasCollection {
    pub fn new() -> Self {
        Self {
            atlases: HashMap::new(),
        }
    }

    pub fn register(&mut self, atlas: Atlas) {
        self.atlases.insert(atlas.id(), atlas);
    }

    pub fn get(&self, id: &AtlasSpriteId) -> KludgineResult<Option<SourceSprite>> {
        match self.atlases.get(&id.0) {
            Some(atlas) => Ok(atlas.get(id)),
            None => Err(KludgineError::InvalidAtlasSpriteId),
        }
    }
}
