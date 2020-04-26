use super::{
    math::{Point, Size},
    scene::SceneTarget,
    sprite::Sprite,
    KludgineResult,
};
use std::mem;

/// TileMap renders tiles retrieved from a TileProvider
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

    pub fn draw(&self, scene: &mut SceneTarget, location: Point<i32>) -> KludgineResult<()> {
        // Normally we don't need to worry about the origin, but in the case of TileMap
        // it will fill the screen with whatever the provider returns for each tile coordinate
        let location = Point::new(
            location.x as f32 + scene.origin().x,
            location.y as f32 + scene.origin().y,
        );

        let tile_height = if let Some(stagger) = &self.stagger {
            stagger.height
        } else {
            self.tile_size.height
        };

        // We need to start at the upper-left of inverting the location
        let min_x = (-location.x / self.tile_size.width as f32).floor() as i32;
        let min_y = (-location.y / self.tile_size.height as f32).floor() as i32;
        let extra_x = (self.tile_size.width - 1) as f32;
        let extra_y = (self.tile_size.height - 1) as f32;
        let total_width = scene.size().width as f32 + extra_x;
        let total_height = scene.size().height as f32 + extra_y;
        let tiles_wide = (total_width / self.tile_size.width as f32).ceil() as i32;
        let tiles_high = (total_height / tile_height as f32).ceil() as i32;

        for y in min_y..(min_y + tiles_high) {
            for x in min_x..(min_x + tiles_wide) {
                let location = Point::new(x, y);
                match self.provider.get_tile(location) {
                    Some(tile) => {
                        let sprite = tile.sprite.get_frame(scene.elapsed())?;
                        sprite.render_at(scene, self.coordinate_for_tile(location));
                    }
                    None => {}
                }
            }
        }

        Ok(())
    }

    fn coordinate_for_tile(&self, location: Point<i32>) -> Point<f32> {
        if let Some(stagger) = &self.stagger {
            let x_stagger = if location.y % 2 == 0 {
                stagger.width as i32
            } else {
                0
            };
            Point::new(
                (location.x * self.tile_size.width as i32 - x_stagger) as f32,
                (location.y * stagger.height as i32) as f32,
            )
        } else {
            Point::new(
                (location.x * self.tile_size.width as i32) as f32,
                (location.y * self.tile_size.height as i32) as f32,
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
            .map_or(None, |tile| {
                tile.as_ref().map_or(None, |tile| Some(tile.clone()))
            })
    }
}

impl PersistentTileProvider {
    pub fn new(size: Size<u32>) -> Self {
        let mut tiles = Vec::new();
        tiles.resize_with((size.width * size.height) as usize, Default::default);
        Self { tiles, size }
    }

    pub fn set(&mut self, location: Point<u32>, sprite: Option<Sprite>) {
        let index = self.point_to_index(location);
        mem::replace(
            &mut self.tiles[index],
            sprite.map_or(None, |sprite| {
                Some(Tile {
                    location: Point::new(location.x as i32, location.y as i32),
                    sprite,
                })
            }),
        );
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
