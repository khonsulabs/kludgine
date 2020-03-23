#[macro_use]
pub extern crate async_trait;
#[allow(unused_imports)]
#[macro_use]
extern crate educe;

pub extern crate glutin;

pub mod application;
pub mod materials;
pub mod math;
pub mod runtime;
pub mod scene2d;
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
}

pub type KludgineResult<T> = Result<T, KludgineError>;

pub mod prelude {
    pub use super::{
        application::Application, glutin, materials::prelude::*, math::*, runtime::Runtime,
        scene2d::prelude::*, window::Window, KludgineError, KludgineResult,
    };
    pub use async_trait::async_trait;
    pub use color_processing::Color;
}

mod internal_prelude {
    pub use super::{math::*, KludgineError, KludgineResult};
    pub use color_processing::Color;
    pub use futures::channel::{mpsc, oneshot};
    pub use futures::executor::block_on;
    pub use futures::sink::SinkExt;
    pub use lazy_static::lazy_static;
}
