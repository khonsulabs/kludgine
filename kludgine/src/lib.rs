#![warn(clippy::all)]

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
