#[macro_use]
pub extern crate async_trait;
#[macro_use]
extern crate educe;

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

use legion::prelude::*;

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

pub mod prelude {
    pub use super::{
        application::{Application, SingleWindowApplication, WindowCreator},
        color::Color,
        glutin::{
            self,
            event::{DeviceId, KeyboardInput, VirtualKeyCode},
        },
        materials::prelude::*,
        math::*,
        runtime::Runtime,
        scene2d::prelude::*,
        shaders::{CompiledProgram, Program, ProgramSource},
        window::Window,
        KludgineError, KludgineResult,
    };
    pub use async_trait::async_trait;
    pub use cgmath::{prelude::*, Deg, Rad};
    pub use legion::prelude::*;
}

mod internal_prelude {
    pub use super::prelude::*;
    pub use futures::channel::{mpsc, oneshot};
    pub use futures::executor::block_on;
    pub use futures::sink::SinkExt;
    pub use lazy_static::lazy_static;
}
