//! Rendering and math related types.

#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    missing_docs,
    // clippy::missing_docs_in_private_items,
    clippy::nursery,
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
pub use easygpu;
pub use figures;
pub use flume;
pub use image;
pub use lazy_static;
pub use winit;

pub use self::{
    color::Color,
    error::Error,
    frame_renderer::{FrameRenderer, ShutdownCallback},
};

/// A collection of commonly used exports provided by this crate.
pub mod prelude {
    pub use figures::{
        Approx as _, Ceil as _, Displayable as _, Floor as _, One as _, Rectlike as _, Round as _,
        Vectorlike as _, Zero as _,
    };

    #[cfg(feature = "bundled-fonts-enabled")]
    pub use super::text::bundled_fonts;
    pub use super::{
        include_aseprite_sprite, include_font, include_texture,
        math::{Angle, Figure, Pixels, Point, Rect, Scale, Scaled, Size, Unknown, Vector},
        scene::{Scene, Target},
        shape::*,
        sprite::{
            AnimationMode, Sprite, SpriteAnimation, SpriteAnimations, SpriteCollection,
            SpriteFrame, SpriteMap, SpriteRotation, SpriteSheet, SpriteSource,
            SpriteSourceSublocation,
        },
        text::{font::Font, prepared::PreparedSpan, Text},
        texture::Texture,
        Color, FrameRenderer, ShutdownCallback,
    };
}

/// Alias for [`std::result::Result`] where the error type is [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
