use super::{
    math::{Point, Scaled, Size},
    scene::SceneTarget,
    sprite::{Sprite, SpriteRotation},
    KludgineResult,
};
use async_trait::async_trait;
use std::{mem, time::Duration};

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

    pub async fn draw(
        &self,
        scene: &SceneTarget,
        location: Point<f32, Scaled>,
    ) -> KludgineResult<()> {
        // Normally we don't need to worry about the origin, but in the case of TileMap
        // it will fill the screen with whatever the provider returns for each tile coordinate
        let location = location + scene.origin().to_vector();

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
        let scene_size = scene.size().await;
        let total_width = scene_size.width + extra_x;
        let total_height = scene_size.height + extra_y;
        let tiles_wide = (total_width / self.tile_size.width as f32).ceil() as i32;
        let tiles_high = (total_height / tile_height as f32).ceil() as i32;

        let elapsed = scene.elapsed().await;

        let mut render_calls = Vec::new();
        for y in min_y..(min_y + tiles_high) {
            for x in min_x..(min_x + tiles_wide) {
                render_calls.push(self.draw_one_tile(x, y, scene, elapsed));
            }
        }

        let _ = futures::future::join_all(render_calls)
            .await
            .into_iter()
            .collect::<Result<_, _>>()?;

        Ok(())
    }

    async fn draw_one_tile(
        &self,
        x: i32,
        y: i32,
        scene: &SceneTarget,
        elapsed: Option<Duration>,
    ) -> KludgineResult<()> {
        let location = Point::new(x, y);
        if let Some(tile) = self.provider.get_tile(location).await {
            let sprite = tile.sprite.get_frame(elapsed).await?;
            sprite
                .render_at(
                    scene,
                    self.coordinate_for_tile(location),
                    SpriteRotation::default(),
                )
                .await;
        }
        Ok(())
    }

    fn coordinate_for_tile(&self, location: Point<i32>) -> Point<f32, Scaled> {
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
#[async_trait]
pub trait TileProvider {
    async fn get_tile(&self, location: Point<i32>) -> Option<Tile>;
}

/// A Tile represents a sprite at an integer offset on the map
#[derive(Clone)]
pub struct Tile {
    pub location: Point<i32>,
    pub sprite: Sprite,
}

/// Provides a simple interface for tile maps that have specific bounds
pub struct PersistentTileProvider {
    tiles: Vec<Option<Tile>>,
    size: Size<u32>,
}

#[async_trait]
impl TileProvider for PersistentTileProvider {
    async fn get_tile(&self, location: Point<i32>) -> Option<Tile> {
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
