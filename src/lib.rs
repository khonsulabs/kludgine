use crossbeam::sync::{ShardedLock, ShardedLockReadGuard, ShardedLockWriteGuard};
use std::sync::{Arc, PoisonError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KludgineError {
    #[error("error sending a WindowMessage to a Window: {0}")]
    InternalWindowMessageSendError(String),
    #[error("error compiling shader: {0}")]
    ShaderCompilationError(String),
    #[error("error reading image: {0}")]
    ImageError(#[from] image::ImageError),
}

pub type KludgineResult<T> = Result<T, KludgineError>;

pub(crate) struct KludgineHandle<T>(Arc<ShardedLock<T>>);

impl<T> KludgineHandle<T> {
    pub fn new(wrapped: T) -> Self {
        Self(Arc::new(ShardedLock::new(wrapped)))
    }

    pub fn write(&self) -> Result<ShardedLockWriteGuard<T>, PoisonError<ShardedLockWriteGuard<T>>> {
        self.0.write()
    }

    pub fn read(&self) -> Result<ShardedLockReadGuard<T>, PoisonError<ShardedLockReadGuard<T>>> {
        self.0.read()
    }
}

impl<T> Clone for KludgineHandle<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub mod application;
pub mod frame;
pub mod math;
pub mod runtime;
pub mod scene;
pub mod sprite;
pub mod text;
pub mod texture;
pub mod timing;
pub mod window;

pub mod prelude {
    pub use super::{
        application::{Application, SingleWindowApplication, WindowCreator},
        math::{Point, Rect, Size, Zeroable},
        runtime::Runtime,
        scene::Scene,
        sprite::{SourceSprite, Sprite},
        texture::Texture,
        window::Window,
        KludgineError, KludgineResult,
    };
    pub use async_trait::async_trait;

    #[cfg(feature = "bundled-fonts-enabled")]
    pub use super::text::bundled_fonts;
}
