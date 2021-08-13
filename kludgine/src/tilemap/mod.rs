use std::{
    mem,
    ops::{Deref, DerefMut},
    time::Duration,
};

use kludgine_core::{
    figures::{Displayable, One, Rectlike, SizedRect},
    math::{ExtentsRect, Figure, Pixels, Point, Scale, Scaled, Size, Unknown},
    scene::Target,
    sprite::{Sprite, SpriteRotation, SpriteSource},
};

/// `TileMap` renders tiles retrieved from a [`TileProvider`]
#[derive(Debug)]
pub struct TileMap<P> {
    provider: P,
    tile_size: Size<u32, Scaled>,
    stagger: Option<Size<u32, Scaled>>,
}

impl<P> TileMap<P>
where
    P: TileProvider,
{
    /// Creates a new tile map of the given size.
    pub fn new(tile_size: Size<u32, Scaled>, provider: P) -> Self {
        Self {
            tile_size,
            provider,
            stagger: None,
        }
    }

    /// Sets the stagger. This causes every odd row of tiles to be offset by
    /// `stagger` when rendered. This is commmonly used when creating isometric
    /// tile maps.
    pub fn set_stagger(&mut self, stagger: Size<u32, Scaled>) {
        self.stagger = Some(stagger);
    }

    /// Renders the tilemap. The tilemap will fill the `target`, but will be
    /// offset by `location`.
    pub fn render(
        &mut self,
        target: &Target,
        location: Point<f32, Scaled>,
    ) -> kludgine_core::Result<()> {
        self.render_scaled(target, location, Scale::one())
    }

    /// Renders the tilemap scaled by `scale`. The tilemap will fill the
    /// `target`, but will be offset by `location`.
    pub fn render_scaled(
        &mut self,
        scene: &Target,
        location: Point<f32, Scaled>,
        scale: Scale<f32, Unknown, Scaled>,
    ) -> kludgine_core::Result<()> {
        let tile_height = if let Some(stagger) = &self.stagger {
            stagger.height
        } else {
            self.tile_size.height
        };
        let tile_size = Size::<u32>::new(self.tile_size.width, tile_height).cast::<f32>() * scale;

        // We need to start at the upper-left of inverting the location
        let min_x = (-location.x / tile_size.width).floor() as i32;
        let min_y = (-location.y / tile_size.height).floor() as i32;
        let extra_x = tile_size.width() - Figure::new(1.);
        let extra_y = tile_size.height() - Figure::new(1.);
        let scene_size = scene.size();
        let total_width = scene_size.width() + extra_x;
        let total_height = scene_size.height() + extra_y;
        let tiles_wide = (total_width / tile_size.width as f32).get().ceil() as i32;
        let tiles_high = (total_height / tile_size.height as f32).get().ceil() as i32;

        let elapsed = scene.elapsed();

        let effective_scale = scene.scale();
        let tile_size = tile_size.to_pixels(effective_scale);
        let render_size = self.tile_size.cast::<f32>()
            * Scale::new(effective_scale.total_scale().get() * scale.get());
        let location = location.to_pixels(effective_scale);
        let mut y_pos = tile_size.height() * min_y as f32 + location.y();
        for y in min_y..(min_y + tiles_high) {
            let mut x_pos = tile_size.width() * min_x as f32 + location.x();
            if let Some(stagger) = &self.stagger {
                if y % 2 == 0 {
                    x_pos -=
                        Figure::<f32, Scaled>::new(stagger.width as f32).to_pixels(effective_scale);
                }
            }
            let next_y = y_pos + tile_size.height();
            for x in min_x..(min_x + tiles_wide) {
                let next_x = x_pos + tile_size.width();
                self.draw_one_tile(
                    Point::new(x, y),
                    SizedRect::new(Point::from_figures(x_pos, y_pos), render_size).as_extents(),
                    scene,
                    elapsed,
                )?;
                x_pos = next_x;
            }
            y_pos = next_y;
        }

        Ok(())
    }

    fn draw_one_tile(
        &mut self,
        tile: Point<i32>,
        destination: ExtentsRect<f32, Pixels>,
        scene: &Target,
        elapsed: Option<Duration>,
    ) -> kludgine_core::Result<()> {
        if let Some(mut tile) = self.provider.tile(tile) {
            let sprite = tile.sprite.get_frame(elapsed)?;
            sprite.render_raw_with_alpha_in_box(scene, destination, SpriteRotation::default(), 1.);
        }
        Ok(())
    }
}

/// `TileProvider` provides [`Tile`]s for a  [`TileMap`].
pub trait TileProvider {
    /// Returns the tile for `location`.
    fn tile(&mut self, location: Point<i32>) -> Option<Tile<'_>>;
}

/// A tile's sprite.
#[derive(Debug)]
pub enum TileSprite<'a> {
    /// A sprite that may be animated.
    Sprite(&'a mut Sprite),
    /// A single frame image.
    SpriteSource(SpriteSource),
}

impl<'a> From<&'a mut Sprite> for TileSprite<'a> {
    fn from(sprite: &'a mut Sprite) -> Self {
        Self::Sprite(sprite)
    }
}

impl<'a> From<SpriteSource> for TileSprite<'a> {
    fn from(sprite: SpriteSource) -> Self {
        Self::SpriteSource(sprite)
    }
}

impl<'a> TileSprite<'a> {
    /// Returns the current frame to display.
    pub fn get_frame(&mut self, elapsed: Option<Duration>) -> kludgine_core::Result<SpriteSource> {
        match self {
            TileSprite::Sprite(sprite) => sprite.get_frame(elapsed),
            TileSprite::SpriteSource(source) => Ok(source.clone()),
        }
    }
}

/// A Tile represents a sprite at an integer offset on the map
#[derive(Debug)]
pub struct Tile<'a> {
    /// The location of the tile.
    pub location: Point<i32>,
    /// The sprite to render for the tile.
    pub sprite: TileSprite<'a>,
}

/// Provides a simple interface for tile maps that have specific bounds
#[derive(Debug)]
pub struct PersistentTileProvider {
    tiles: Vec<Option<PersistentTileSource>>,
    dimensions: Size<u32>,
}

/// A tile sprite source for [`PersistentTileProvider`].
#[derive(Debug)]
pub enum PersistentTileSource {
    /// A sprite.
    Sprite(Sprite),
    /// A sprite source.
    SpriteSource(SpriteSource),
}

impl From<Sprite> for PersistentTileSource {
    fn from(sprite: Sprite) -> Self {
        Self::Sprite(sprite)
    }
}

impl<'a> From<SpriteSource> for PersistentTileSource {
    fn from(sprite: SpriteSource) -> Self {
        Self::SpriteSource(sprite)
    }
}

impl PersistentTileSource {
    fn as_tile(&mut self, location: Point<i32>) -> Tile<'_> {
        Tile {
            location,
            sprite: match self {
                PersistentTileSource::Sprite(sprite) => TileSprite::Sprite(sprite),
                PersistentTileSource::SpriteSource(source) =>
                    TileSprite::SpriteSource(source.clone()),
            },
        }
    }
}

impl TileProvider for PersistentTileProvider {
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    fn tile(&mut self, location: Point<i32>) -> Option<Tile<'_>> {
        if location.x < 0
            || location.y < 0
            || location.x >= self.dimensions.width as i32
            || location.y >= self.dimensions.height as i32
        {
            return None;
        }

        let index = self.point_to_index(Point::new(location.x as u32, location.y as u32));
        self.tiles
            .get_mut(index)
            .and_then(|tile| tile.as_mut().map(|t| t.as_tile(location)))
    }
}

impl PersistentTileProvider {
    /// Returns a blank map with dimensions `dimensions`.
    #[must_use]
    pub fn blank(dimensions: Size<u32>) -> Self {
        let mut tiles = Vec::new();
        tiles.resize_with((dimensions.width * dimensions.height) as usize, || {
            Option::<Sprite>::None
        });
        Self::new(dimensions, tiles)
    }

    /// Creates a new map using `tiles` with `dimensions`. Tiles are initialized
    /// from left-to-right then top-to-bottom.
    #[must_use]
    pub fn new<S: Into<PersistentTileSource>>(
        dimensions: Size<u32>,
        tiles: Vec<Option<S>>,
    ) -> Self {
        let tiles = tiles
            .into_iter()
            .map(|sprite| sprite.map(Into::into))
            .collect();
        Self { tiles, dimensions }
    }

    /// Sets a single tile at `location`. Returns the existing tile, if one was
    /// set.
    ///
    /// # Panics
    ///
    /// Panics if `location` is outside of the bounds of this map.
    pub fn set<I: Into<PersistentTileSource>>(
        &mut self,
        location: Point<u32>,
        sprite: Option<I>,
    ) -> Option<PersistentTileSource> {
        let index = self.point_to_index(location);
        mem::replace(&mut self.tiles[index], sprite.map(Into::into))
    }

    const fn point_to_index(&self, location: Point<u32>) -> usize {
        (location.x + location.y * self.dimensions.width) as usize
    }
}

/// `PersistentTileMap` is an alias for
/// [`TileMap`]`<`[`PersistentTileProvider`]`>`
pub type PersistentTileMap = TileMap<PersistentTileProvider>;

/// Convenience trait for creating persistent tile maps.
pub trait PersistentMap {
    /// Creates a [`TileMap`] using a [`PersistentTileProvider`].
    ///
    /// # Arguments
    ///
    /// * `tile_size`: The dimensions of each tile
    /// * `map_size`: The size of the map, in number of tiles
    fn persistent_with_size(tile_size: Size<u32, Scaled>, map_size: Size<u32>) -> Self;

    /// Sets a single tile at `location`. Returns the existing tile, if one was
    /// set.
    ///
    /// # Panics
    ///
    /// Panics if `location` is outside of the bounds of this map.
    fn set<I: Into<PersistentTileSource>>(&mut self, location: Point<u32>, sprite: Option<I>);
}

impl PersistentMap for PersistentTileMap {
    fn persistent_with_size(tile_size: Size<u32, Scaled>, map_size: Size<u32>) -> Self {
        Self::new(tile_size, PersistentTileProvider::blank(map_size))
    }

    fn set<I: Into<PersistentTileSource>>(&mut self, location: Point<u32>, sprite: Option<I>) {
        self.provider.set(location, sprite.map(Into::into));
    }
}

impl<T> Deref for TileMap<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.provider
    }
}

impl<T> DerefMut for TileMap<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.provider
    }
}
