#![warn(clippy::all)]

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
