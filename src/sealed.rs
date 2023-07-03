use std::sync::atomic::{self, AtomicUsize};
use std::sync::{Arc, OnceLock};

use crate::Graphics;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct TextureId(usize);

impl TextureId {
    pub fn new_unique_id() -> Self {
        static COUNTER: OnceLock<AtomicUsize> = OnceLock::new();
        Self(
            COUNTER
                .get_or_init(AtomicUsize::default)
                .fetch_add(1, atomic::Ordering::Relaxed),
        )
    }
}

pub trait ShaderScalableSealed {
    fn flags() -> u32;
}
pub trait TextureSource {
    fn id(&self) -> TextureId;
    fn bind_group(&self, graphics: &Graphics<'_>) -> Arc<wgpu::BindGroup>;
}
