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
)]

mod application;
mod error;
mod runtime;
mod window;
#[cfg(feature = "multiwindow")]
pub use window::OpenableWindow;

pub use self::{
    application::{Application, SingleWindowApplication},
    error::Error,
    runtime::Runtime,
    window::{
        event, RedrawRequester, RedrawStatus, Window, WindowBuilder, WindowCreator, WindowHandle,
    },
};

/// A collection of commonly used exports provided by this crate.
pub mod prelude {
    pub use super::{
        event::{
            DeviceId, ElementState, Event, EventStatus, InputEvent, MouseButton, MouseScrollDelta,
            ScanCode, TouchPhase, VirtualKeyCode,
        },
        Application, Error, RedrawRequester, RedrawStatus, Runtime, SingleWindowApplication,
        Window, WindowBuilder, WindowCreator, WindowHandle,
    };
}

/// Alias for [`std::result::Result`] where the error type is [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
