use crate::sprite::{SpriteSheet, SpriteSource};
use async_handle::Handle;
use async_trait::async_trait;
use std::{collections::HashMap, fmt::Debug, hash::Hash};

#[derive(Debug, Clone)]
pub struct SpriteMap<T> {
    data: Handle<SpriteMapData<T>>,
}

impl<T> Default for SpriteMap<T> {
    fn default() -> Self {
        SpriteMap {
            data: Handle::new(SpriteMapData {
                sprites: HashMap::new(),
            }),
        }
    }
}

#[derive(Debug, Clone)]
struct SpriteMapData<T> {
    sprites: HashMap<T, SpriteSource>,
}

impl<T> SpriteMap<T>
where
    T: Debug + Eq + Hash,
{
    pub fn new(sprites: HashMap<T, SpriteSource>) -> Self {
        Self {
            data: Handle::new(SpriteMapData { sprites }),
        }
    }

    pub async fn from_foreign_sheet<O: Clone + Debug + Eq + Hash, F: Fn(O) -> T>(
        sheet: SpriteSheet<O>,
        converter: F,
    ) -> Self {
        let map = Self::default();
        map.add_foreign_sheet(sheet, converter).await;
        map
    }

    pub async fn add_foreign_sheet<O: Clone + Debug + Eq + Hash, F: Fn(O) -> T>(
        &self,
        sheet: SpriteSheet<O>,
        converter: F,
    ) {
        let mut data = self.data.write().await;
        for (tile, sprite) in sheet.all_sprites().await {
            data.sprites.insert(converter(tile), sprite);
        }
    }
}

impl<T> SpriteMap<T>
where
    T: Clone + Debug + Eq + Hash,
{
    pub async fn add_sheet(&self, sheet: SpriteSheet<T>) {
        self.add_foreign_sheet(sheet, |a| a).await
    }

    pub async fn keys(&self) -> Vec<T> {
        let data = self.data.read().await;
        data.sprites.keys().cloned().collect()
    }
}

#[async_trait]
pub trait SpriteCollection<T> {
    async fn sprite(&self, tile: &T) -> Option<SpriteSource>;
}

#[async_trait]
impl<T> SpriteCollection<T> for SpriteMap<T>
where
    T: Send + Sync + Eq + Hash,
{
    async fn sprite(&self, tile: &T) -> Option<SpriteSource> {
        let data = self.data.read().await;
        data.sprites.get(tile).cloned()
    }
}
