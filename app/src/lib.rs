//! Application and window handling.

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
)]

pub mod application;
mod error;
pub mod runtime;
pub mod window;
pub use error::Error;

pub mod prelude {
    #[cfg(feature = "multiwindow")]
    pub use super::window::OpenableWindow;
    pub use super::{
        application::{Application, SingleWindowApplication},
        runtime::Runtime,
        window::{
            event::{
                DeviceId, ElementState, Event, EventStatus, InputEvent, MouseButton,
                MouseScrollDelta, ScanCode, TouchPhase, VirtualKeyCode,
            },
            OpenWindow, RedrawStatus, Window, WindowBuilder, WindowCreator,
        },
    };
}

/// Alias for [`std::result::Result`] where the eroor type is [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
