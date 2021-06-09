//! 2d graphics and app framework built atop wgpu.

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
    clippy::multiple_crate_versions, // this is a mess due to winit dependencies and wgpu dependencies not lining up
)]

/// Types for rendering tilemaps.
pub mod tilemap;
#[cfg(feature = "app")]
#[doc(inline)]
pub use kludgine_app as app;
#[doc(inline)]
pub use kludgine_core as core;

cfg_if::cfg_if! {
    if #[cfg(feature = "app")] {
        pub use app::Result as Result;
    } else {
        pub use core::Result as Result;
    }
}

/// Convenience module that exports the public interface of Kludgine
pub mod prelude {
    #[cfg(feature = "app")]
    pub use super::app::prelude::*;
    pub use super::{
        core::prelude::*,
        tilemap::{
            PersistentMap, PersistentTileMap, PersistentTileProvider, Tile, TileMap, TileProvider,
        },
    };
}
