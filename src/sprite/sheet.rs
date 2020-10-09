use crate::{
    math::Size,
    math::{Point, Rect},
    sprite::{SpriteCollection, SpriteMap, SpriteSource},
    texture::Texture,
};
use async_handle::Handle;
use async_trait::async_trait;
use std::{collections::HashMap, fmt::Debug, hash::Hash};

#[derive(Debug, Clone)]
pub struct SpriteSheet<T>
where
    T: Debug,
{
    pub texture: Texture,
    data: Handle<SpriteSheetData<T>>,
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
    pub async fn new(texture: Texture, tile_size: Size<u32>, tiles: Vec<T>) -> Self {
        let dimensions = divide_size(texture.size().cast(), tile_size);
        Self {
            texture,
            data: Handle::new(SpriteSheetData::from_tiles(tiles, tile_size, dimensions)),
        }
    }

    pub async fn tile_size(&self) -> Size<u32> {
        let data = self.data.read().await;
        data.tile_size
    }

    pub async fn sprites<I: IntoIterator<Item = T>>(&self, iterator: I) -> Vec<SpriteSource> {
        let data = self.data.read().await;
        iterator
            .into_iter()
            .map(|tile| {
                let location = data.sprites.get(&tile).unwrap();
                SpriteSource::new(*location, self.texture.clone())
            })
            .collect()
    }

    pub async fn sprite_map<I: IntoIterator<Item = T>>(&self, iterator: I) -> SpriteMap<T> {
        let data = self.data.read().await;
        let map = iterator
            .into_iter()
            .map(|tile| {
                let location = data.sprites.get(&tile).unwrap();
                (tile, SpriteSource::new(*location, self.texture.clone()))
            })
            .collect::<HashMap<_, _>>();
        SpriteMap::new(map)
    }
}

impl<T> SpriteSheet<T>
where
    T: Clone + Debug + Eq + Hash,
{
    pub async fn all_sprites(&self) -> HashMap<T, SpriteSource> {
        let data = self.data.read().await;
        data.sprites
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

#[async_trait]
impl<T> SpriteCollection<T> for SpriteSheet<T>
where
    T: Debug + Send + Sync + Eq + Hash,
{
    async fn sprite(&self, tile: &T) -> Option<SpriteSource> {
        let data = self.data.read().await;
        let location = data.sprites.get(tile);
        location.map(|location| SpriteSource::new(*location, self.texture.clone()))
    }
}
