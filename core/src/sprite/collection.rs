use std::{
    collections::{hash_map, HashMap},
    fmt::Debug,
    hash::Hash,
    ops::Deref,
};

use crate::sprite::{SpriteSheet, SpriteSource};

/// A collection of [`SpriteSource`]s.
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
    /// Creates a new collection with `sprites`.
    #[must_use]
    pub fn new(sprites: HashMap<T, SpriteSource>) -> Self {
        Self { sprites }
    }

    /// Creates a collection from `sheet` using `converter` to convert from `O`
    /// to `T`.
    #[must_use]
    pub fn from_foreign_sheet<O: Clone + Debug + Eq + Hash, F: Fn(O) -> T>(
        sheet: &SpriteSheet<O>,
        converter: F,
    ) -> Self {
        let mut map = Self::default();
        map.add_foreign_sheet(sheet, converter);
        map
    }

    /// Adds a collection from `sheet` using `converter` to convert from `O` to
    /// `T`.
    pub fn add_foreign_sheet<O: Clone + Debug + Eq + Hash, F: Fn(O) -> T>(
        &mut self,
        sheet: &SpriteSheet<O>,
        converter: F,
    ) {
        for (tile, sprite) in sheet.to_sprite_map() {
            self.sprites.insert(converter(tile), sprite);
        }
    }
}

impl<T> SpriteMap<T>
where
    T: Clone + Debug + Eq + Hash,
{
    /// Adds all sprites from `sheet`.
    pub fn add_sheet(&mut self, sheet: &SpriteSheet<T>) {
        self.add_foreign_sheet(sheet, |a| a);
    }
}

impl<T> Deref for SpriteMap<T> {
    type Target = HashMap<T, SpriteSource>;

    fn deref(&self) -> &HashMap<T, SpriteSource> {
        &self.sprites
    }
}

impl<T> IntoIterator for SpriteMap<T> {
    type IntoIter = hash_map::IntoIter<T, SpriteSource>;
    type Item = (T, SpriteSource);

    fn into_iter(self) -> Self::IntoIter {
        self.sprites.into_iter()
    }
}

/// A collection of sprites.
pub trait SpriteCollection<T>
where
    T: Send + Sync,
{
    /// Returns the sprite referred to by `tile`.
    #[must_use]
    fn sprite(&self, tile: &T) -> Option<SpriteSource>;

    /// Returns all of the requested `tiles`.
    ///
    /// # Panics
    ///
    /// Panics if a tile is not found.
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
