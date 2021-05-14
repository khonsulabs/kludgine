use crate::{
    math::{Point, Rect, Size},
    sprite::{SpriteCollection, SpriteMap, SpriteSource},
    texture::Texture,
};
use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};

#[derive(Debug, Clone)]
pub struct SpriteSheet<T>
where
    T: Debug,
{
    pub texture: Texture,
    data: Arc<SpriteSheetData<T>>,
}

#[derive(Debug)]
struct SpriteSheetData<T>
where
    T: Debug,
{
    tile_size: Size<u32>,
    dimensions: Size<u32>,
    sprites: HashMap<T, Rect<u32>>,
}

impl<T> SpriteSheet<T>
where
    T: Debug + Eq + Hash,
{
    pub fn new(texture: Texture, tile_size: Size<u32>, tiles: Vec<T>) -> Self {
        let dimensions = divide_size(texture.size().cast(), tile_size);
        Self {
            texture,
            data: Arc::new(SpriteSheetData::from_tiles(tiles, tile_size, dimensions)),
        }
    }

    pub fn tile_size(&self) -> Size<u32> {
        self.data.tile_size
    }

    pub fn sprites<I: IntoIterator<Item = T>>(&self, iterator: I) -> Vec<SpriteSource> {
        iterator
            .into_iter()
            .map(|tile| {
                let location = self.data.sprites.get(&tile).unwrap();
                SpriteSource::new(*location, self.texture.clone())
            })
            .collect()
    }

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

fn divide_size(a: Size<u32>, b: Size<u32>) -> Size<u32> {
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

        Self {
            dimensions,
            sprites,
            tile_size,
        }
    }
}

impl<T> SpriteSheet<T>
where
    T: Clone + Debug + Eq + Hash,
{
    pub fn all_sprites(&self) -> HashMap<T, SpriteSource> {
        self.data
            .sprites
            .iter()
            .map(|(tile, location)| {
                (
                    tile.clone(),
                    SpriteSource::new(*location, self.texture.clone()),
                )
            })
            .collect()
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
