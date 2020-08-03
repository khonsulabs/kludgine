use super::{
    math::{Point, Points, Size},
    scene::SceneTarget,
    sprite::Sprite,
    KludgineResult,
};
use std::mem;

/// TileMap renders tiles retrieved from a TileProvider
#[derive(Debug)]
pub struct TileMap<P> {
    provider: P,
    tile_size: Size<u32>,
    stagger: Option<Size<u32>>,
}

impl<P> TileMap<P>
where
    P: TileProvider,
{
    pub fn new(tile_size: Size<u32>, provider: P) -> Self {
        Self {
            tile_size,
            provider,
            stagger: None,
        }
    }

    pub fn set_stagger(&mut self, stagger: Size<u32>) {
        self.stagger = Some(stagger);
    }

    pub async fn draw(&self, scene: &SceneTarget, location: Point<Points>) -> KludgineResult<()> {
        // Normally we don't need to worry about the origin, but in the case of TileMap
        // it will fill the screen with whatever the provider returns for each tile coordinate
        let location = Point::new(location.x + scene.origin().x, location.y + scene.origin().y);

        let tile_height = if let Some(stagger) = &self.stagger {
            stagger.height
        } else {
            self.tile_size.height
        };

        // We need to start at the upper-left of inverting the location
        let min_x = (-location.x.to_f32() / self.tile_size.width as f32).floor() as i32;
        let min_y = (-location.y.to_f32() / self.tile_size.height as f32).floor() as i32;
        let extra_x = (self.tile_size.width - 1) as f32;
        let extra_y = (self.tile_size.height - 1) as f32;
        let scene_size = scene.size().await;
        let total_width = scene_size.width.to_f32() + extra_x;
        let total_height = scene_size.height.to_f32() + extra_y;
        let tiles_wide = (total_width / self.tile_size.width as f32).ceil() as i32;
        let tiles_high = (total_height / tile_height as f32).ceil() as i32;

        let elapsed = scene.elapsed().await;

        for y in min_y..(min_y + tiles_high) {
            for x in min_x..(min_x + tiles_wide) {
                let location = Point::new(x, y);
                if let Some(tile) = self.provider.get_tile(location) {
                    let sprite = tile.sprite.get_frame(elapsed).await?;
                    sprite
                        .render_at(scene, self.coordinate_for_tile(location))
                        .await;
                }
            }
        }

        Ok(())
    }

    fn coordinate_for_tile(&self, location: Point<i32>) -> Point<Points> {
        if let Some(stagger) = &self.stagger {
            let x_stagger = if location.y % 2 == 0 {
                stagger.width as i32
            } else {
                0
            };
            Point::new(
                Points((location.x * self.tile_size.width as i32 - x_stagger) as f32),
                Points((location.y * stagger.height as i32) as f32),
            )
        } else {
            Point::new(
                Points((location.x * self.tile_size.width as i32) as f32),
                Points((location.y * self.tile_size.height as i32) as f32),
            )
        }
    }
}

/// TileProvider is how a TileMap retrieves tiles to render
pub trait TileProvider {
    fn get_tile(&self, location: Point<i32>) -> Option<Tile>;
}

/// A Tile represents a sprite at an integer offset on the map
#[derive(Clone)]
pub struct Tile {
    location: Point<i32>,
    sprite: Sprite,
}

/// Provides a simple interface for tile maps that have specific bounds
pub struct PersistentTileProvider {
    tiles: Vec<Option<Tile>>,
    size: Size<u32>,
}

impl TileProvider for PersistentTileProvider {
    fn get_tile(&self, location: Point<i32>) -> Option<Tile> {
        if location.x < 0
            || location.y < 0
            || location.x >= self.size.width as i32
            || location.y >= self.size.height as i32
        {
            return None;
        }

        self.tiles
            .get(self.point_to_index(Point::new(location.x as u32, location.y as u32)))
            .and_then(|tile| tile.clone())
    }
}

impl PersistentTileProvider {
    pub fn new(size: Size<u32>) -> Self {
        let mut tiles = Vec::new();
        tiles.resize_with((size.width * size.height) as usize, Default::default);
        Self { tiles, size }
    }

    pub fn set(&mut self, location: Point<u32>, sprite: Option<Sprite>) -> Option<Tile> {
        let index = self.point_to_index(location);
        mem::replace(
            &mut self.tiles[index],
            sprite.map(|sprite| Tile {
                location: Point::new(location.x as i32, location.y as i32),
                sprite,
            }),
        )
    }

    fn point_to_index(&self, location: Point<u32>) -> usize {
        (location.x + location.y * self.size.width) as usize
    }
}

/// PersistentTileMap is an alias for TileMap<PersistentTileProvider>
pub type PersistentTileMap = TileMap<PersistentTileProvider>;

pub trait PersistentMap {
    fn persistent_with_size(tile_size: Size<u32>, map_size: Size<u32>) -> Self;

    fn set(&mut self, location: Point<u32>, sprite: Option<Sprite>);
}

impl PersistentMap for PersistentTileMap {
    /// Creates a TileMap using a PersistentTileProvider
    ///
    /// # Arguments
    ///
    /// * `tile_size`: THe dimensions of each tile
    /// * `map_size`: The size of the map, in number of tiles
    fn persistent_with_size(tile_size: Size<u32>, map_size: Size<u32>) -> Self {
        TileMap::new(tile_size, PersistentTileProvider::new(map_size))
    }

    fn set(&mut self, location: Point<u32>, sprite: Option<Sprite>) {
        self.provider.set(location, sprite);
    }
}
