#![deny(clippy::all)]
#![allow(clippy::map_entry)]
use thiserror::Error;

#[cfg(test)]
#[macro_use]
extern crate futures_await_test;

// Re-exports
pub use async_handle::Handle;
pub use async_trait::async_trait;
pub use clipboard;
pub use easygpu;
pub use winit;

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
    #[error("clipboard error: {0}")]
    Clipboard(String),

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

#[macro_use]
mod internal_macros {

    #[macro_export]
    macro_rules! hash_map {
        ($($key:expr => $value:expr),+ $(,)*) => {{
            let mut map = std::collections::HashMap::new();
            $(
                map.insert($key, $value);
            )+
            map
        }};
    }

    #[macro_export]
    macro_rules! hash_set {
        ($($value:expr),+ $(,)*) => {{
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
mod ext;
pub mod math;
pub mod runtime;
pub mod scene;
pub mod shape;
pub mod sprite;
pub mod style;
pub mod text;
pub mod texture;
pub mod tilemap;
pub mod ui;
pub mod window;

/// Convenience module that exports the public interface of Kludgine
pub mod prelude {
    pub use super::{
        application::{Application, SingleWindowApplication},
        color::Color,
        include_aseprite_sprite, include_font, include_texture,
        math::{
            Angle, Dimension, Length, Pixels, Point, PointExt, Points, Raw, Rect, Scale, Scaled,
            ScreenScale, Size, SizeExt, Surround, Unknown, Vector,
        },
        runtime::Runtime,
        scene::{Scene, Target},
        shape::*,
        sprite::{
            AnimationMode, Sprite, SpriteAnimation, SpriteAnimations, SpriteCollection,
            SpriteFrame, SpriteMap, SpriteRotation, SpriteSheet, SpriteSource,
            SpriteSourceSublocation,
        },
        style::{
            theme::{
                ColorGroup, ElementKind, Intent, Minimal, Palette, PaletteShade, SystemTheme,
                Theme, VariableColor,
            },
            Alignment, AnyStyleComponent, BackgroundColor, ColorPair, ComponentCollection,
            FontFamily, FontSize, FontStyle, ForegroundColor, GenericStyle, Style, StyleComponent,
            StyleSheet, UnscaledStyleComponent, VerticalAlignment, Weight,
        },
        text::{
            font::Font,
            rich::{RichText, RichTextPosition},
            wrap::TextWrap,
            Span, Text,
        },
        texture::Texture,
        tilemap::{
            PersistentMap, PersistentTileMap, PersistentTileProvider, Tile, TileMap, TileProvider,
        },
        ui::{
            AbsoluteBounds, AbsoluteLayout, AnimatableComponent, AnimationManager, Border, Button,
            Callback, ColumnLayout, Component, ComponentBorder, ComponentOverflow, ContentOffset,
            Context, ControlEvent, Dialog, DialogButton, DialogButtonSpacing, DialogButtons,
            Entity, EntityBuilder, Grid, GridCommand, GridEvent, HierarchicalArena, Image,
            ImageAlphaAnimation, ImageCommand, ImageFrameAnimation, ImageOptions, ImageScaling,
            Index, Indexable, InteractiveComponent, InteractiveComponentExt, Label, LabelCommand,
            Layout, LayoutConstraints, LayoutContext, LayoutSolver, LayoutSolverExt,
            LinearTransition, Overflow, Pane, Panel, PanelCommand, PanelEvent, PanelMessage,
            PanelProvider, RowLayout, Scroll, ScrollCommand, ScrollEvent, ScrollGutterColor,
            Scrollbar, ScrollbarCommand, ScrollbarGripColor, ScrollbarMetrics, ScrollbarSize,
            StandaloneComponent, StyledContext, TextField, TextFieldEvent, Timeout, Toast,
        },
        window::{
            event::{
                DeviceId, ElementState, Event, EventStatus, InputEvent, MouseButton,
                MouseScrollDelta, ScanCode, TouchPhase, VirtualKeyCode,
            },
            OpenableWindow, Window, WindowBuilder, WindowCreator,
        },
        Handle, KludgineError, KludgineResult, RequiresInitialization,
    };
    pub use async_trait::async_trait;
    pub use winit::event::*;

    #[cfg(feature = "bundled-fonts-enabled")]
    pub use super::text::bundled_fonts;
}

pub struct RequiresInitialization<T>(Option<T>);

impl<T> RequiresInitialization<T> {
    pub fn initialize_with(&mut self, value: T) {
        assert!(self.0.is_none());
        self.0 = Some(value);
    }

    pub fn unwrap(self) -> T {
        self.0.unwrap()
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
