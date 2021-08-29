use figures::{Displayable, Rectlike, Scaled};

use crate::{
    math::{ExtentsRect, Pixels, Point, Rect, Size},
    scene::{Element, Target},
    sprite::{RenderedSprite, SpriteRotation},
    texture::Texture,
};

/// A sprite's source location and texture. Cheap to clone.
#[derive(Debug, Clone)]
pub struct SpriteSource {
    /// The location of the sprite
    pub location: SpriteSourceLocation,
    /// The texture.
    pub texture: Texture,
}

/// A sprite location.
#[derive(Debug, Clone)]
pub enum SpriteSourceLocation {
    /// A single rectangle.
    Rect(Rect<u32>),
    /// A joined series of images. Useful for constructing a 32x32 sprite from
    /// four 16x16 sprites.
    Joined(Vec<SpriteSourceSublocation>),
}

impl SpriteSourceLocation {
    /// Returns the bounding box of the source rect.
    #[must_use]
    pub fn bounds(&self) -> Rect<u32> {
        match self {
            Self::Rect(rect) => *rect,
            Self::Joined(locations) => locations
                .iter()
                .fold(Option::<Rect<u32>>::None, |union, location| {
                    union.map_or_else(
                        || Some(location.destination_rect()),
                        |total| {
                            total
                                .union(&location.destination_rect())
                                .map(|r| r.as_sized())
                        },
                    )
                })
                .unwrap_or_default(),
        }
    }

    /// Returns the size of the bounds.
    #[must_use]
    pub fn size(&self) -> Size<u32> {
        self.bounds().size
    }
}

/// A sub-location of a joined sprite.
#[derive(Debug, Clone)]
pub struct SpriteSourceSublocation {
    /// The source rectangle.
    pub source: Rect<u32>,
    /// The relative destination when rendering.
    pub destination: Point<u32>,
}

impl SpriteSourceSublocation {
    /// Returns the destination with the source's size.
    #[must_use]
    pub const fn destination_rect(&self) -> Rect<u32> {
        Rect::new(self.destination, self.source.size)
    }
}

impl SpriteSource {
    /// Creates a new sprite source with the location and textuer given.
    #[must_use]
    pub const fn new(location: Rect<u32>, texture: Texture) -> Self {
        Self {
            location: SpriteSourceLocation::Rect(location),
            texture,
        }
    }

    /// Creates a sprite by joining multiple rectangular areas from `texture`
    /// into one drawable sprite.
    #[must_use]
    pub fn joined<I: IntoIterator<Item = SpriteSourceSublocation>>(
        locations: I,
        texture: Texture,
    ) -> Self {
        Self {
            location: SpriteSourceLocation::Joined(locations.into_iter().collect()),
            texture,
        }
    }

    /// Creates a sprite by joining an iterator of `SpriteSource`s into one. All
    /// `SpriteSources` must be from the same texture, and the iterator must
    /// have a square number of sprites.
    #[must_use]
    pub fn joined_square<I: IntoIterator<Item = Self>>(sources: I) -> Self {
        let sources: Vec<_> = sources.into_iter().collect();
        #[allow(clippy::cast_sign_loss)] // sqrt of a positive number is always positive
        let sprites_wide = (sources.len() as f32).sqrt() as usize;
        assert!(sprites_wide * sprites_wide == sources.len()); // check for square
        let texture = sources[0].texture.clone();

        let sprite_size = sources[0].location.bounds().size;
        let mut sources = sources.into_iter();
        let mut locations = Vec::new();
        for y in 0..sprites_wide {
            for x in 0..sprites_wide {
                let source = sources.next().unwrap();
                debug_assert!(texture.id() == source.texture.id());
                locations.push(SpriteSourceSublocation {
                    source: source.location.bounds(),
                    destination: Point::new(
                        x as u32 * sprite_size.width,
                        y as u32 * sprite_size.height,
                    ),
                });
            }
        }

        Self::joined(locations, texture)
    }

    /// Creates a sprite source for an entire texture.
    #[must_use]
    pub fn entire_texture(texture: Texture) -> Self {
        Self::new(Rect::new(Point::default(), texture.size()), texture)
    }

    /// Renders the sprite at `location` with `rotation` into `scene`.
    pub fn render_at(
        &self,
        scene: &Target,
        location: impl Displayable<f32, Pixels = Point<f32, Pixels>>,
        rotation: impl Displayable<f32, Pixels = SpriteRotation<Pixels>>,
    ) {
        self.render_at_with_alpha(scene, location, rotation, 1.);
    }

    /// Renders the sprite within `bounds` (stretching if needed) with
    /// `rotation` into `scene`.
    pub fn render_within(
        &self,
        scene: &Target,
        bounds: impl Displayable<f32, Pixels = Rect<f32, Pixels>>,
        rotation: impl Displayable<f32, Pixels = SpriteRotation<Pixels>>,
    ) {
        self.render_with_alpha(scene, bounds, rotation, 1.);
    }

    /// Renders the sprite with `alpha` at `location` with `rotation` into
    /// `scene`.
    #[allow(clippy::needless_pass_by_value)]
    pub fn render_at_with_alpha(
        &self,
        scene: &Target,
        location: impl Displayable<f32, Pixels = Point<f32, Pixels>>,
        rotation: impl Displayable<f32, Pixels = SpriteRotation<Pixels>>,
        alpha: f32,
    ) {
        self.render_with_alpha(
            scene,
            Rect::new(
                location.to_pixels(scene.scale()),
                self.location
                    .size()
                    .cast::<f32>()
                    .cast_unit::<Scaled>()
                    .to_pixels(scene.scale()),
            ),
            rotation,
            alpha,
        );
    }

    /// Renders the sprite with `alpha` within `bounds` with `rotation` into
    /// `scene`.
    #[allow(clippy::needless_pass_by_value)]
    pub fn render_with_alpha(
        &self,
        scene: &Target,
        bounds: impl Displayable<f32, Pixels = Rect<f32, Pixels>>,
        rotation: impl Displayable<f32, Pixels = SpriteRotation<Pixels>>,
        alpha: f32,
    ) {
        self.render_with_alpha_in_box(
            scene,
            bounds.to_pixels(scene.scale()).as_extents(),
            rotation,
            alpha,
        );
    }

    /// Renders the sprite with `alpha` within `bounds` with `rotation` into
    /// `scene`.
    #[allow(clippy::needless_pass_by_value)]
    pub fn render_with_alpha_in_box(
        &self,
        scene: &Target,
        bounds: impl Displayable<f32, Pixels = ExtentsRect<f32, Pixels>>,
        rotation: impl Displayable<f32, Pixels = SpriteRotation<Pixels>>,
        alpha: f32,
    ) {
        let effective_scale = scene.scale();
        self.render_raw_with_alpha_in_box(
            scene,
            bounds.to_pixels(effective_scale),
            rotation.to_pixels(effective_scale),
            alpha,
        );
    }

    /// Renders the sprite with `alpha` within `bounds` with `rotation` into
    /// `scene`.
    #[allow(clippy::needless_pass_by_value)]
    pub fn render_raw_with_alpha_in_box(
        &self,
        scene: &Target,
        bounds: impl Displayable<f32, Pixels = ExtentsRect<f32, Pixels>>,
        rotation: impl Displayable<f32, Pixels = SpriteRotation<Pixels>>,
        alpha: f32,
    ) {
        let bounds = bounds.to_pixels(scene.scale());
        let bounds = ExtentsRect::new(
            scene.offset_point_raw(bounds.origin),
            scene.offset_point_raw(bounds.extent),
        );
        scene.push_element(Element::Sprite {
            sprite: RenderedSprite::new(
                bounds,
                rotation.to_pixels(scene.scale()),
                alpha,
                self.clone(),
            ),
            clip: scene.clip,
        });
    }
}
