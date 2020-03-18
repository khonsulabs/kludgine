#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate educe;

pub extern crate glium;
pub extern crate glutin;

pub mod application;
pub mod material;
pub mod runtime;
pub mod scene2d;

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
    #[error("error receiving response from channel")]
    InternalCommunicationError(#[from] futures::channel::oneshot::Canceled),
}

pub type KludgineResult<T> = Result<T, KludgineError>;

pub mod prelude {
    pub use super::{
        application::Application, glium, glutin, material::Material, runtime::Runtime,
        scene2d::prelude::*, KludgineError, KludgineResult,
    };
    pub use async_trait::async_trait;
    pub use color_processing::Color;
}

mod internal_prelude {
    pub use super::{KludgineError, KludgineResult};
    pub use color_processing::Color;
    pub use futures::channel::{mpsc, oneshot};
    pub use futures::executor::block_on;
    pub use futures::sink::SinkExt;
    pub use lazy_static::lazy_static;
}
