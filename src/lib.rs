#![deny(clippy::all)]
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
    #[error("error tessellating shape")]
    TessellationError(lyon_tessellation::TessellationError),
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
pub use async_handle::Handle;

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
pub mod color;
pub mod event;
pub mod math;
pub mod runtime;
pub mod scene;
pub mod shape;
pub mod sprite;
pub mod style;
pub mod text;
pub mod texture;
pub mod theme;
pub mod tilemap;
pub mod ui;
pub mod window;

/// Convenience module that exports the public interface of Kludgine
pub mod prelude {
    pub use super::{
        application::{Application, SingleWindowApplication},
        color::Color,
        event::*,
        include_aseprite_sprite, include_font, include_texture,
        math::{
            Angle, Dimension, Length, Pixels, Point, PointExt, Points, Raw, Rect, Scale, Scaled,
            ScreenScale, Size, SizeExt, Surround, Unknown, Vector,
        },
        runtime::Runtime,
        scene::{Scene, SceneTarget},
        shape::*,
        sprite::{Sprite, SpriteRotation, SpriteSource},
        style::*,
        text::{font::Font, wrap::TextWrap, Span, Text},
        texture::Texture,
        theme::{ColorGroup, ElementKind, Intent, Palette, PaletteShade, Theme, VariableColor},
        tilemap::{
            PersistentMap, PersistentTileMap, PersistentTileProvider, Tile, TileMap, TileProvider,
        },
        ui::{
            AbsoluteBounds, AbsoluteLayout, AnimatableComponent, AnimationManager, Button,
            ButtonStyle, Callback, Component, Context, ControlEvent, Entity, EntityBuilder,
            HierarchicalArena, Image, ImageAlphaAnimation, ImageCommand, ImageFrameAnimation,
            ImageOptions, ImageScaling, Index, Indexable, InteractiveComponent, Label,
            LabelCommand, Layout, LayoutConstraints, LayoutContext, LinearTransition, SceneContext,
            StandaloneComponent, StyledContext, Timeout,
        },
        window::{Event, EventStatus, InputEvent, OpenableWindow, Window, WindowCreator},
        Handle, KludgineError, KludgineResult, RequiresInitialization,
    };
    pub use async_trait::async_trait;
    pub use winit::event::*;

    #[cfg(feature = "bundled-fonts-enabled")]
    pub use super::text::bundled_fonts;

    pub use lazy_static::lazy_static;
}

pub struct RequiresInitialization<T>(Option<T>);

impl<T> RequiresInitialization<T> {
    pub fn initialize_with(&mut self, value: T) {
        assert!(self.0.is_none());
        self.0 = Some(value);
    }
}

impl<T> Default for RequiresInitialization<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T: Clone> Clone for RequiresInitialization<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Copy> Copy for RequiresInitialization<T> {}

impl<T> From<T> for RequiresInitialization<T> {
    fn from(value: T) -> Self {
        Self(Some(value))
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for RequiresInitialization<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> RequiresInitialization<T> {
    pub fn new(initialized: T) -> Self {
        Self(Some(initialized))
    }
}

impl<T> std::ops::Deref for RequiresInitialization<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("used without initializing")
    }
}

impl<T> std::ops::DerefMut for RequiresInitialization<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().expect("used without initializing")
    }
}
