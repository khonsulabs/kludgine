use std::{collections::HashMap, fmt::Debug, hash::Hash};

use crate::sprite::{SpriteSheet, SpriteSource};

#[derive(Debug, Clone)]
pub struct SpriteMap<T> {
    sprites: HashMap<T, SpriteSource>,
}

impl<T> Default for SpriteMap<T> {
    fn default() -> Self {
        Self {
            sprites: HashMap::default(),
        }
    }
}

impl<T> SpriteMap<T>
where
    T: Debug + Eq + Hash,
{
    #[must_use]
    pub fn new(sprites: HashMap<T, SpriteSource>) -> Self {
        Self { sprites }
    }

    #[must_use]
    pub fn from_foreign_sheet<O: Clone + Debug + Eq + Hash, F: Fn(O) -> T>(
        sheet: &SpriteSheet<O>,
        converter: F,
    ) -> Self {
        let mut map = Self::default();
        map.add_foreign_sheet(sheet, converter);
        map
    }

    pub fn add_foreign_sheet<O: Clone + Debug + Eq + Hash, F: Fn(O) -> T>(
        &mut self,
        sheet: &SpriteSheet<O>,
        converter: F,
    ) {
        for (tile, sprite) in sheet.all_sprites() {
            self.sprites.insert(converter(tile), sprite);
        }
    }
}

impl<T> SpriteMap<T>
where
    T: Clone + Debug + Eq + Hash,
{
    pub fn add_sheet(&mut self, sheet: &SpriteSheet<T>) {
        self.add_foreign_sheet(sheet, |a| a)
    }

    #[must_use]
    pub fn keys(&self) -> Vec<&'_ T> {
        self.sprites.keys().collect()
    }
}

pub trait SpriteCollection<T>
where
    T: Send + Sync,
{
    #[must_use]
    fn sprite(&self, tile: &T) -> Option<SpriteSource>;

    #[must_use]
    fn sprites(&self, tiles: &[T]) -> Vec<SpriteSource> {
        tiles
            .iter()
            .map(|t| self.sprite(t).unwrap())
            .collect::<Vec<_>>()
    }
}

impl<T> SpriteCollection<T> for SpriteMap<T>
where
    T: Send + Sync + Eq + Hash,
{
    fn sprite(&self, tile: &T) -> Option<SpriteSource> {
        self.sprites.get(tile).cloned()
    }
}
