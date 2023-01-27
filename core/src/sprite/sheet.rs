use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use crate::math::{Point, Rect, Size};
use crate::sprite::{SpriteCollection, SpriteMap, SpriteSource};
use crate::texture::Texture;

/// A collection of sprites from a single [`Texture`].
#[derive(Debug, Clone)]
pub struct SpriteSheet<T>
where
    T: Debug,
{
    /// The source texture.
    pub texture: Texture,
    data: Arc<SpriteSheetData<T>>,
}

#[derive(Debug)]
struct SpriteSheetData<T>
where
    T: Debug,
{
    tile_size: Size<u32>,
    sprites: HashMap<T, Rect<u32>>,
}

impl<T> SpriteSheet<T>
where
    T: Debug + Eq + Hash,
{
    /// Creates a new sprite sheet, diving `texture` into a grid of `tile_size`.
    /// The order of `tiles` will be read left-to-right, top-to-bottom.
    #[must_use]
    pub fn new(texture: Texture, tile_size: Size<u32>, tiles: Vec<T>) -> Self {
        let dimensions = divide_size(texture.size().cast(), tile_size);
        Self {
            texture,
            data: Arc::new(SpriteSheetData::from_tiles(tiles, tile_size, dimensions)),
        }
    }

    /// Returns the size of the tiles within this sheet.
    #[must_use]
    pub fn tile_size(&self) -> Size<u32> {
        self.data.tile_size
    }

    /// Returns the sprites identified by each element in `iterator`.
    ///
    /// # Panics
    ///
    /// Panics if a tile isn't found.
    #[must_use]
    pub fn sprites<I: IntoIterator<Item = T>>(&self, iterator: I) -> Vec<SpriteSource> {
        iterator
            .into_iter()
            .map(|tile| {
                let location = self.data.sprites.get(&tile).unwrap();
                SpriteSource::new(*location, self.texture.clone())
            })
            .collect()
    }

    /// Returns the sprites identified by each element in `iterator` into a
    /// [`SpriteMap`].
    #[must_use]
    pub fn sprite_map<I: IntoIterator<Item = T>>(&self, iterator: I) -> SpriteMap<T> {
        let map = iterator
            .into_iter()
            .map(|tile| {
                let location = self.data.sprites.get(&tile).unwrap();
                (tile, SpriteSource::new(*location, self.texture.clone()))
            })
            .collect::<HashMap<_, _>>();
        SpriteMap::new(map)
    }
}

const fn divide_size(a: Size<u32>, b: Size<u32>) -> Size<u32> {
    Size::new(a.width / b.width, a.height / b.height)
}

impl<T: Debug + Eq + Hash> SpriteSheetData<T> {
    fn from_tiles(tiles: Vec<T>, tile_size: Size<u32>, dimensions: Size<u32>) -> Self {
        let mut sprites = HashMap::new();

        for (index, tile) in tiles.into_iter().enumerate() {
            let index = index as u32;
            let y = index / dimensions.width;
            let x = index - y * dimensions.width;
            sprites.insert(
                tile,
                Rect::new(
                    Point::new(x * tile_size.width, y * tile_size.height),
                    tile_size,
                ),
            );
        }

        Self { tile_size, sprites }
    }
}

impl<T> SpriteSheet<T>
where
    T: Clone + Debug + Eq + Hash,
{
    /// Returns a collection of all tiles in the sheet  as
    #[must_use]
    pub fn to_sprite_map(&self) -> SpriteMap<T> {
        SpriteMap::new(
            self.data
                .sprites
                .clone()
                .iter()
                .map(|(tile, location)| {
                    (
                        tile.clone(),
                        SpriteSource::new(*location, self.texture.clone()),
                    )
                })
                .collect(),
        )
    }
}

impl<T> SpriteCollection<T> for SpriteSheet<T>
where
    T: Debug + Send + Sync + Eq + Hash,
{
    fn sprite(&self, tile: &T) -> Option<SpriteSource> {
        let location = self.data.sprites.get(tile);
        location.map(|location| SpriteSource::new(*location, self.texture.clone()))
    }
}
