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

    pub async fn read<'a>(&'a self) -> RwLockReadGuard<'a, T> {
        self.handle.read().await
    }

    pub async fn write<'a>(&'a self) -> RwLockWriteGuard<'a, T> {
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
        text::{wrap::TextWrap, Span, Text},
        texture::Texture,
        tilemap::{
            PersistentMap, PersistentTileMap, PersistentTileProvider, TileMap, TileProvider,
        },
        ui::{
            grid::Grid, label::Label, Component, ComponentEventStatus, Controller, UserInterface,
        },
        window::{Event, EventStatus, InputEvent, Window},
        KludgineError, KludgineHandle, KludgineResult,
    };
    pub use async_trait::async_trait;
    pub use winit::event::*;

    #[cfg(feature = "bundled-fonts-enabled")]
    pub use super::text::bundled_fonts;

    pub use lazy_static::lazy_static;
}
