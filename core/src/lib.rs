//! Rendering and math related types.

#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    missing_docs,
    // clippy::missing_docs_in_private_items,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms,
)]
#![cfg_attr(doc, deny(rustdoc::all))]
#![allow(
    clippy::missing_errors_doc, // TODO clippy::missing_errors_doc
    clippy::missing_panics_doc, // TODO clippy::missing_panics_doc
    clippy::option_if_let_else,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::mut_mut, // false alarm on futures::select!
)]

mod color;
mod delay;
mod error;
/// Renders individual frames. Can be used for offscreen rendering.
pub mod frame_renderer;
/// Math types for 2d geometry.
pub mod math;
/// `Scene` and `Target` types that are used to draw.
pub mod scene;
/// Types for rendering shapes.
pub mod shape;
/// Types for rendering sprites.
pub mod sprite;
#[cfg(test)]
mod tests;
/// Types for rendering text.
pub mod text;
/// Types for managing textures.
pub mod texture;

// Re-exports
pub use {easygpu, figures, flume, image, lazy_static, winit};

pub use self::color::Color;
pub use self::error::Error;
pub use self::frame_renderer::{FrameRenderer, ShutdownCallback};

/// A collection of commonly used exports provided by this crate.
pub mod prelude {
    pub use figures::{
        Approx as _, Ceil as _, Displayable as _, Floor as _, One as _, Rectlike as _, Round as _,
        Vectorlike as _, Zero as _,
    };

    pub use super::math::{
        Angle, Figure, Pixels, Point, Rect, Scale, Scaled, Size, Unknown, Vector,
    };
    pub use super::scene::{Scene, Target};
    pub use super::shape::*;
    pub use super::sprite::{
        AnimationMode, Sprite, SpriteAnimation, SpriteAnimations, SpriteCollection, SpriteFrame,
        SpriteMap, SpriteRotation, SpriteSheet, SpriteSource, SpriteSourceSublocation,
    };
    #[cfg(feature = "bundled-fonts-enabled")]
    pub use super::text::bundled_fonts;
    pub use super::text::font::Font;
    pub use super::text::prepared::PreparedSpan;
    pub use super::text::Text;
    pub use super::texture::Texture;
    pub use super::{
        include_aseprite_sprite, include_font, include_texture, Color, FrameRenderer,
        ShutdownCallback,
    };
}

/// Alias for [`std::result::Result`] where the error type is [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
