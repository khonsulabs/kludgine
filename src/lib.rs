#[macro_use]
pub extern crate async_trait;
#[macro_use]
extern crate educe;

pub extern crate glutin;
pub extern crate image;

pub mod application;
pub mod color;
pub mod materials;
pub mod math;
pub mod runtime;
pub mod scene2d;
pub mod shaders;
pub mod texture;
pub mod window;

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
    InvalidId(generational_arena::Index),
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
        glutin,
        materials::prelude::*,
        math::*,
        runtime::Runtime,
        scene2d::prelude::*,
        window::Window,
        KludgineError, KludgineResult,
    };
    pub use async_trait::async_trait;
    pub use cgmath::{prelude::*, Deg, Rad};
}

mod internal_prelude {
    pub use super::prelude::*;
    pub use futures::channel::{mpsc, oneshot};
    pub use futures::executor::block_on;
    pub use futures::sink::SinkExt;
    pub use lazy_static::lazy_static;
}
