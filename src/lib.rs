#[macro_use]
pub extern crate async_trait;
#[macro_use]
extern crate educe;

pub extern crate crossbeam;
pub extern crate glutin;
pub extern crate image;
pub extern crate legion;

pub mod application;
pub mod color;
pub mod materials;
pub mod math;
pub mod runtime;
pub mod scene2d;
pub mod shaders;
pub mod texture;
pub mod window;

use crossbeam::sync::ShardedLock;
use legion::prelude::*;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum KludgineError {
    #[error("error sending a WindowMessage to a Window: {0}")]
    InternalWindowMessageSendError(String),
    #[error("The id could not be found: {0:?}")]
    InvalidId(Entity),
    #[error("error compiling shader: {0}")]
    ShaderCompilationError(String),
    #[error("error reading image: {0}")]
    ImageError(#[from] image::ImageError),
}

pub type KludgineResult<T> = Result<T, KludgineError>;

pub type KludgineHandle<T> = Arc<ShardedLock<T>>;

pub trait HandleMethods<T> {
    fn wrap(wrapped: T) -> KludgineHandle<T>;
}

impl<T> HandleMethods<T> for KludgineHandle<T> {
    fn wrap(wrapped: T) -> KludgineHandle<T> {
        Arc::new(ShardedLock::new(wrapped))
    }
}

pub mod prelude {
    pub use super::{
        application::{Application, SingleWindowApplication, WindowCreator},
        color::Color,
        glutin::{
            self,
            event::{DeviceId, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode},
        },
        materials::prelude::*,
        math::*,
        runtime::Runtime,
        scene2d::prelude::*,
        shaders::{CompiledProgram, Program, ProgramSource},
        window::{Event, InputEvent, Window},
        HandleMethods, KludgineError, KludgineHandle, KludgineResult,
    };
    pub use async_trait::async_trait;
    pub use cgmath::{prelude::*, Deg, Rad};
    pub use legion::prelude::*;
}

mod internal_prelude {
    pub use super::prelude::*;
    pub use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
    pub use futures::executor::block_on;
    pub use futures::sink::SinkExt;
    pub use lazy_static::lazy_static;
    pub use std::sync::Arc;
}
