use crate::{
    math::{Point, Raw, Rect, Scaled, Size},
    scene::{Element, Scene},
    sprite::{RenderedSprite, SpriteRotation},
    texture::Texture,
    Handle,
};
use euclid::Box2D; // TODO Expose this in crate::math
#[derive(Debug, Clone)]
pub struct SpriteSource {
    pub(crate) handle: Handle<SpriteSourceData>,
}

#[derive(Debug, Clone)]
pub enum SpriteSourceLocation {
    Rect(Rect<u32>),
    Joined(Vec<SpriteSourceSublocation>),
}

impl SpriteSourceLocation {
    pub fn bounds(&self) -> Rect<u32> {
        match self {
            Self::Rect(rect) => *rect,
            Self::Joined(locations) => locations
                .iter()
                .fold(Option::<Rect<u32>>::None, |union, location| {
                    Some(
                        union
                            .map(|total| total.union(&location.destination_rect()))
                            .unwrap_or_else(|| location.destination_rect()),
                    )
                })
                .unwrap_or_default(),
        }
    }

    pub fn size(&self) -> Size<u32> {
        self.bounds().size
    }
}

#[derive(Debug, Clone)]
pub struct SpriteSourceSublocation {
    pub source: Rect<u32>,
    pub destination: Point<u32>,
}

impl SpriteSourceSublocation {
    pub fn destination_rect(&self) -> Rect<u32> {
        Rect::new(self.destination, self.source.size)
    }
}

#[derive(Debug)]
pub(crate) struct SpriteSourceData {
    pub location: SpriteSourceLocation,
    pub texture: Texture,
}

impl SpriteSource {
    pub fn new(location: Rect<u32>, texture: Texture) -> Self {
        SpriteSource {
            handle: Handle::new(SpriteSourceData {
                location: SpriteSourceLocation::Rect(location),
                texture,
            }),
        }
    }

    pub fn joined<I: IntoIterator<Item = SpriteSourceSublocation>>(
        locations: I,
        texture: Texture,
    ) -> Self {
        Self {
            handle: Handle::new(SpriteSourceData {
                location: SpriteSourceLocation::Joined(locations.into_iter().collect()),
                texture,
            }),
        }
    }

    /// All SpriteSources must be from the same texture, and must have a square number of sprites
    pub async fn joined_square<I: IntoIterator<Item = SpriteSource>>(sources: I) -> Self {
        let sources: Vec<_> = sources.into_iter().collect();
        let sprites_wide = (sources.len() as f32).sqrt() as usize;
        assert!(sprites_wide * sprites_wide == sources.len()); // check for square
        let texture = sources[0].texture().await;
        let texture_id = texture.id().await;
        let sprite_size = sources[0].location().await.bounds().size;
        let mut sources = sources.into_iter();
        let mut locations = Vec::new();
        for y in 0..sprites_wide {
            for x in 0..sprites_wide {
                let source = sources.next().unwrap();
                debug_assert!(texture_id == source.texture().await.id().await);
                locations.push(SpriteSourceSublocation {
                    source: source.location().await.bounds(),
                    destination: Point::new(
                        x as u32 * sprite_size.width,
                        y as u32 * sprite_size.height,
                    ),
                });
            }
        }

        Self::joined(locations, texture)
    }

    pub async fn entire_texture(texture: Texture) -> Self {
        let (w, h) = {
            let texture = texture.handle.read().await;
            (texture.image.width(), texture.image.height())
        };
        Self::new(Rect::new(Point::default(), Size::new(w, h)), texture)
    }

    pub async fn render_at(
        &self,
        scene: &Scene,
        location: Point<f32, Scaled>,
        rotation: SpriteRotation<Scaled>,
    ) {
        self.render_at_with_alpha(scene, location, rotation, 1.)
            .await
    }

    pub async fn render_within(
        &self,
        scene: &Scene,
        bounds: Rect<f32, Scaled>,
        rotation: SpriteRotation<Scaled>,
    ) {
        self.render_with_alpha(scene, bounds, rotation, 1.).await
    }

    pub async fn render_within_box(
        &self,
        scene: &Scene,
        bounds: Box2D<f32, Scaled>,
        rotation: SpriteRotation<Scaled>,
    ) {
        self.render_with_alpha_in_box(scene, bounds, rotation, 1.)
            .await
    }

    pub async fn render_at_with_alpha(
        &self,
        scene: &Scene,
        location: Point<f32, Scaled>,
        rotation: SpriteRotation<Scaled>,
        alpha: f32,
    ) {
        let sprite_location = self.location().await;
        self.render_with_alpha(
            scene,
            Rect::new(location, sprite_location.size().to_f32().cast_unit()),
            rotation,
            alpha,
        )
        .await
    }

    pub async fn render_with_alpha(
        &self,
        scene: &Scene,
        bounds: Rect<f32, Scaled>,
        rotation: SpriteRotation<Scaled>,
        alpha: f32,
    ) {
        self.render_with_alpha_in_box(scene, bounds.to_box2d(), rotation, alpha)
            .await
    }

    pub async fn render_with_alpha_in_box(
        &self,
        scene: &Scene,
        bounds: Box2D<f32, Scaled>,
        rotation: SpriteRotation<Scaled>,
        alpha: f32,
    ) {
        let effective_scale = scene.scale_factor().await;
        self.render_raw_with_alpha_in_box(
            scene,
            bounds * effective_scale,
            rotation * effective_scale,
            alpha,
        )
        .await
    }

    pub async fn render_raw_with_alpha_in_box(
        &self,
        scene: &Scene,
        bounds: Box2D<f32, Raw>,
        rotation: SpriteRotation<Raw>,
        alpha: f32,
    ) {
        scene
            .push_element(Element::Sprite(RenderedSprite::new(
                bounds,
                rotation,
                alpha,
                self.clone(),
            )))
            .await;
    }

    pub async fn location(&self) -> SpriteSourceLocation {
        let sprite = self.handle.read().await;
        sprite.location.clone()
    }

    pub async fn texture(&self) -> Texture {
        let sprite = self.handle.read().await;
        sprite.texture.clone()
    }
}
