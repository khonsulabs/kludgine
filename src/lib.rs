#[macro_use]
extern crate async_trait;

pub extern crate glium;
pub extern crate glutin;

pub mod application;
pub mod runtime;

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
        application::Application,
        glium, glutin,
        runtime::{Runtime, RuntimeHandleMethods},
    };
    pub use async_trait::async_trait;
}

mod internal_prelude {
    pub use super::{KludgineError, KludgineResult};
    pub use futures::executor::block_on;
    pub use lazy_static::lazy_static;
}
