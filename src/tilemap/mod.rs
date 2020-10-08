use crate::{math::Raw, math::Unknown, sprite::SpriteSource};

use super::{
    math::{Point, PointExt, Scaled, Size, SizeExt},
    scene::Scene,
    sprite::{Sprite, SpriteRotation},
    KludgineResult,
};
use async_trait::async_trait;
use euclid::{Box2D, Scale};
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

    pub async fn draw(&self, scene: &Scene, location: Point<f32, Scaled>) -> KludgineResult<()> {
        self.draw_scaled(scene, location, Scale::identity()).await
    }

    pub async fn draw_scaled(
        &self,
        scene: &Scene,
        location: Point<f32, Scaled>,
        scale: Scale<f32, Unknown, Scaled>,
    ) -> KludgineResult<()> {
        let tile_height = if let Some(stagger) = &self.stagger {
            stagger.height
        } else {
            self.tile_size.height
        };
        let tile_size = Size::<u32>::new(self.tile_size.width, tile_height).cast::<f32>() * scale;

        // We need to start at the upper-left of inverting the location
        let min_x = (-location.x / tile_size.width).floor() as i32;
        let min_y = (-location.y / tile_size.height).floor() as i32;
        let extra_x = tile_size.width - 1.;
        let extra_y = tile_size.height - 1.;
        let scene_size = scene.size().await;
        let total_width = scene_size.width + extra_x;
        let total_height = scene_size.height + extra_y;
        let tiles_wide = (total_width / tile_size.width as f32).ceil() as i32;
        let tiles_high = (total_height / tile_height as f32).ceil() as i32;

        let elapsed = scene.elapsed().await;

        let mut render_calls = Vec::new();
        let effective_scale = scene.scale_factor().await;
        let tile_size = tile_size * effective_scale;
        let location = location * effective_scale;
        let mut y_pos = tile_size.height() * min_y as f32 + location.y();
        for y in min_y..(min_y + tiles_high) {
            let mut x_pos = tile_size.width() * min_x as f32 + location.x();
            let next_y = y_pos + tile_size.height();
            for x in min_x..(min_x + tiles_wide) {
                let next_x = x_pos + tile_size.width();
                render_calls.push(
                    self.draw_one_tile(
                        Point::new(x, y),
                        Box2D::new(
                            Point::from_lengths(x_pos, y_pos),
                            Point::from_lengths(next_x, next_y),
                        )
                        .round(),
                        scene,
                        elapsed,
                    ),
                );
                x_pos = next_x;
            }
            y_pos = next_y;
        }

        let _ = futures::future::join_all(render_calls)
            .await
            .into_iter()
            .collect::<Result<_, _>>()?;

        Ok(())
    }

    async fn draw_one_tile(
        &self,
        tile: Point<i32>,
        destination: Box2D<f32, Raw>,
        scene: &Scene,
        elapsed: Option<Duration>,
    ) -> KludgineResult<()> {
        if let Some(tile) = self.provider.get_tile(tile).await {
            let sprite = tile.sprite.get_frame(elapsed).await?;
            sprite
                .render_raw_with_alpha_in_box(scene, destination, SpriteRotation::default(), 1.)
                .await;
        }
        Ok(())
    }
}

/// TileProvider is how a TileMap retrieves tiles to render
#[async_trait]
pub trait TileProvider {
    async fn get_tile(&self, location: Point<i32>) -> Option<Tile>;
}

#[derive(Debug, Clone)]
pub enum TileSprite {
    Sprite(Sprite),
    SpriteSource(SpriteSource),
}

impl From<Sprite> for TileSprite {
    fn from(sprite: Sprite) -> Self {
        Self::Sprite(sprite)
    }
}

impl From<SpriteSource> for TileSprite {
    fn from(sprite: SpriteSource) -> Self {
        Self::SpriteSource(sprite)
    }
}

impl TileSprite {
    pub async fn get_frame(&self, elapsed: Option<Duration>) -> KludgineResult<SpriteSource> {
        match self {
            TileSprite::Sprite(sprite) => sprite.get_frame(elapsed).await,
            TileSprite::SpriteSource(source) => Ok(source.clone()),
        }
    }
}

/// A Tile represents a sprite at an integer offset on the map
#[derive(Debug, Clone)]
pub struct Tile {
    pub location: Point<i32>,
    pub sprite: TileSprite,
}

/// Provides a simple interface for tile maps that have specific bounds
#[derive(Debug)]
pub struct PersistentTileProvider {
    tiles: Vec<Option<Tile>>,
    dimensions: Size<u32>,
}

#[async_trait]
impl TileProvider for PersistentTileProvider {
    async fn get_tile(&self, location: Point<i32>) -> Option<Tile> {
        if location.x < 0
            || location.y < 0
            || location.x >= self.dimensions.width as i32
            || location.y >= self.dimensions.height as i32
        {
            return None;
        }

        self.tiles
            .get(self.point_to_index(Point::new(location.x as u32, location.y as u32)))
            .and_then(|tile| tile.clone())
    }
}

impl PersistentTileProvider {
    pub fn blank(size: Size<u32>) -> Self {
        let mut tiles = Vec::new();
        tiles.resize_with((size.width * size.height) as usize, || {
            Option::<Sprite>::None
        });
        Self::new(size, tiles)
    }

    pub fn new<S: Into<TileSprite>>(dimensions: Size<u32>, tiles: Vec<Option<S>>) -> Self {
        let tiles = tiles
            .into_iter()
            .enumerate()
            .map(|(index, sprite)| {
                sprite.map(|sprite| {
                    let dimensions = dimensions.cast::<i32>();
                    let index = index as i32;
                    let y = index / dimensions.width;

                    Tile {
                        location: Point::new(index - y * dimensions.width, y),
                        sprite: sprite.into(),
                    }
                })
            })
            .collect();
        Self { tiles, dimensions }
    }

    pub fn set<I: Into<TileSprite>>(
        &mut self,
        location: Point<u32>,
        sprite: Option<I>,
    ) -> Option<Tile> {
        let index = self.point_to_index(location);
        mem::replace(
            &mut self.tiles[index],
            sprite.map(|sprite| Tile {
                location: Point::new(location.x as i32, location.y as i32),
                sprite: sprite.into(),
            }),
        )
    }

    fn point_to_index(&self, location: Point<u32>) -> usize {
        (location.x + location.y * self.dimensions.width) as usize
    }
}

/// PersistentTileMap is an alias for TileMap<PersistentTileProvider>
pub type PersistentTileMap = TileMap<PersistentTileProvider>;

pub trait PersistentMap {
    fn persistent_with_size(tile_size: Size<u32>, map_size: Size<u32>) -> Self;

    fn set<I: Into<TileSprite>>(&mut self, location: Point<u32>, sprite: Option<I>);
}

impl PersistentMap for PersistentTileMap {
    /// Creates a TileMap using a PersistentTileProvider
    ///
    /// # Arguments
    ///
    /// * `tile_size`: THe dimensions of each tile
    /// * `map_size`: The size of the map, in number of tiles
    fn persistent_with_size(tile_size: Size<u32>, map_size: Size<u32>) -> Self {
        TileMap::new(tile_size, PersistentTileProvider::blank(map_size))
    }

    fn set<I: Into<TileSprite>>(&mut self, location: Point<u32>, sprite: Option<I>) {
        self.provider.set(location, sprite.map(|s| s.into()));
    }
}
