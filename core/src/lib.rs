#![warn(clippy::all)]

pub mod color;
mod delay;
mod error;
pub mod math;
pub mod renderer;
pub mod scene;
pub mod shape;
pub mod sprite;
#[cfg(test)]
mod tests;
pub mod text;
pub mod texture;

// Re-exports
pub use easygpu;
pub use euclid;
pub use flume;
pub use lazy_static;
pub use winit;

pub use self::error::Error;

pub mod prelude {
    #[cfg(feature = "bundled-fonts-enabled")]
    pub use super::text::bundled_fonts;
    pub use super::{
        color::Color,
        include_aseprite_sprite, include_font, include_texture,
        math::{
            Angle, Dimension, Length, Pixels, Point, PointExt, Points, Raw, Rect, Scale, Scaled,
            ScreenScale, Size, SizeExt, Surround, Unknown, Vector,
        },
        scene::{Scene, Target},
        shape::*,
        sprite::{
            AnimationMode, Sprite, SpriteAnimation, SpriteAnimations, SpriteCollection,
            SpriteFrame, SpriteMap, SpriteRotation, SpriteSheet, SpriteSource,
            SpriteSourceSublocation,
        },
        text::{font::Font, prepared::PreparedSpan, Text},
        texture::Texture,
    };
}

/// Alias for [`std::result::Result`] where the eroor type is [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
