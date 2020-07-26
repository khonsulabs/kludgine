#![deny(clippy::all)]
use async_std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use futures::executor::block_on;
use std::{fmt::Display, sync::Arc};
use thiserror::Error;

#[cfg(test)]
#[macro_use]
extern crate futures_await_test;

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
    #[error("An index provided was not found")]
    InvalidIndex,
    #[error("error parsing sprite data: {0}")]
    SpriteParseError(String),
    #[error("no frames could be found for the current tag")]
    InvalidSpriteTag,
    #[error("font family not found: {0}")]
    FontFamilyNotFound(String),
    #[error("argument is out of bounds")]
    OutOfBounds,

    #[error("specify at most 2 of the dimensions top, bottom, and height. (e.g., top and bottom, but not height")]
    AbsoluteBoundsInvalidVertical,
    #[error("specify at most 2 of the dimensions left, right, and width. (e.g., left and right, but not width)")]
    AbsoluteBoundsInvalidHorizontal,

    #[error("other error: {0}")]
    Other(#[from] anyhow::Error),
}
/// Alias for [`Result<T,E>`] where `E` is [`KludgineError`]
///
/// [`Result<T,E>`]: http://doc.rust-lang.org/std/result/enum.Result.html
/// [`KludgineError`]: enum.KludgineError.html
pub type KludgineResult<T> = Result<T, KludgineError>;

#[derive(Debug)]
pub struct KludgineHandle<T> {
    handle: Arc<RwLock<T>>,
}

impl<T> KludgineHandle<T> {
    pub fn new(wrapped: T) -> Self {
        Self {
            handle: Arc::new(RwLock::new(wrapped)),
        }
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.handle.read().await
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.handle.write().await
    }
}

impl<T> Clone for KludgineHandle<T> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}

impl<T> Display for KludgineHandle<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KludgineHandle<")?;
        let inner = block_on(self.handle.read());
        inner.fmt(f)?;
        write!(f, ">")
    }
}

impl<T> Default for KludgineHandle<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[macro_use]
mod internal_macros {

    #[macro_export]
    macro_rules! hash_map {
        ($($key:expr => $value:expr),+) => {{
            let mut map = std::collections::HashMap::new();
            $(
                map.insert($key, $value);
            )+
            map
        }};
    }

    #[macro_export]
    macro_rules! hash_set {
        ($($value:expr),+) => {{
            let mut set = std::collections::HashSet::new();
            $(
                set.insert($value);
            )+
            set
        }};
    }
}

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

pub use rgx::kit::shape2d as shape;

/// Convenience module that exports the public interface of Kludgine
pub mod prelude {
    pub use super::{
        application::{Application, SingleWindowApplication, WindowCreator},
        include_aseprite_sprite, include_font, include_texture,
        math::{Dimension, Point, Rect, Size, Surround},
        runtime::Runtime,
        scene::{Scene, SceneTarget},
        shape::*,
        source_sprite::SourceSprite,
        sprite::Sprite,
        style::*,
        text::{font::Font, wrap::TextWrap, Span, Text},
        texture::Texture,
        tilemap::{
            PersistentMap, PersistentTileMap, PersistentTileProvider, TileMap, TileProvider,
        },
        timing::FrequencyLimiter,
        ui::*,
        window::{Event, EventStatus, InputEvent, OpenableWindow, Window},
        KludgineError, KludgineHandle, KludgineResult,
    };
    pub use async_trait::async_trait;
    pub use winit::event::*;

    #[cfg(feature = "bundled-fonts-enabled")]
    pub use super::text::bundled_fonts;

    pub use lazy_static::lazy_static;
}
