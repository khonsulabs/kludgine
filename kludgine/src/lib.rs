use futures::lock::Mutex;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KludgineError {
    #[error("error sending a WindowMessage to a Window: {0}")]
    InternalWindowMessageSendError(String),
    #[error("error reading image: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("error parsing json: {0}")]
    JsonError(#[from] json::Error),
    #[error("AtlasSpriteId belongs to an Atlas not registered in this collection")]
    InvalidAtlasSpriteId,
    #[error("error parsing sprite data: {0}")]
    SpriteParseError(String),
    #[error("no frames could be found for the current tag")]
    InvalidSpriteTag,
    #[error("font family not found: {0}")]
    FontFamilyNotFound(String),
    #[error("argument is out of bounds")]
    OutOfBounds,
}
/// Alias for [`Result<T,E>`] where `E` is [`KludgineError`]
///
/// [`Result<T,E>`]: http://doc.rust-lang.org/std/result/enum.Result.html
/// [`KludgineError`]: enum.KludgineError.html
pub type KludgineResult<T> = Result<T, KludgineError>;

pub type KludgineHandle<T> = Arc<Mutex<T>>;

pub mod application;
pub mod frame;
pub mod math;
pub mod runtime;
pub mod scene;
pub mod source_sprite;
pub mod sprite;
pub mod style;
pub mod text;
pub mod texture;
pub mod tilemap;
pub mod timing;
pub mod ui;
pub mod window;

/// Convenience module that exports the public interface of Kludgine
pub mod prelude {
    pub use super::{
        application::{Application, SingleWindowApplication, WindowCreator},
        include_aseprite_sprite,
        math::{Dimension, Point, Rect, Size, Surround},
        runtime::Runtime,
        scene::{Scene, SceneTarget},
        source_sprite::SourceSprite,
        sprite::Sprite,
        style::*,
        text::{Span, Text, TextWrap},
        texture::Texture,
        tilemap::{
            PersistentMap, PersistentTileMap, PersistentTileProvider, TileMap, TileProvider,
        },
        ui::{
            grid::Grid,
            label::Label,
            view::{View, ViewBuilder, ViewCore, ViewCoreBuilder},
            Component, Controller, UserInterface,
        },
        window::{Event, InputEvent, Window},
        KludgineError, KludgineResult,
    };
    pub use async_trait::async_trait;
    pub use winit::event::*;

    #[cfg(feature = "bundled-fonts-enabled")]
    pub use super::text::bundled_fonts;
}
