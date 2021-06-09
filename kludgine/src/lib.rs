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
    missing_docs,
    clippy::multiple_crate_versions, // this is a mess due to winit dependencies and wgpu dependencies not lining up
)]

pub mod tilemap;
#[cfg(feature = "app")]
#[doc(inline)]
pub use kludgine_app as app;
#[doc(inline)]
pub use kludgine_core as core;

/// Convenience module that exports the public interface of Kludgine
pub mod prelude {
    pub use super::tilemap::{
        PersistentMap, PersistentTileMap, PersistentTileProvider, Tile, TileMap, TileProvider,
    };
    cfg_if::cfg_if! {
        if #[cfg(feature = "app")] {
            pub use super::app::prelude::*;
            pub use super::app::Result as KludgineResult;
        } else {
            pub use super::core::Result as KludgineResult;
        }
    }
    pub use super::core::prelude::*;
}
