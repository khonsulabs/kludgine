use crossbeam::sync::ShardedLock;
use legion::prelude::*;
use std::sync::Arc;
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

pub mod application;
pub mod math;
pub mod runtime;
pub mod scene;
pub mod window;

pub mod prelude {
    pub use super::{
        application::{Application, SingleWindowApplication, WindowCreator},
        runtime::Runtime,
        scene::Scene,
        window::Window,
        KludgineError, KludgineHandle, KludgineResult,
    };
    pub use async_trait::async_trait;
}
