use std::sync::atomic::{self, AtomicUsize};
use std::sync::{Arc, OnceLock};

use figures::units::UPx;
use figures::Rect;
use smallvec::smallvec;

use crate::buffer::Buffer;
use crate::pipeline::{PreparedCommand, Vertex};
use crate::{Graphics, PreparedGraphic};

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
    fn is_mask(&self) -> bool;
    fn bind_group(&self) -> Arc<wgpu::BindGroup>;
    fn default_rect(&self) -> Rect<UPx>;
}

pub trait ShapeSource<Unit> {
    fn vertices(&self) -> &[Vertex<Unit>];
    fn indices(&self) -> &[u16];
    fn prepare(
        &self,
        texture: Option<&impl TextureSource>,
        graphics: &Graphics<'_>,
    ) -> PreparedGraphic<Unit>
    where
        Vertex<Unit>: bytemuck::Pod,
    {
        let vertices = Buffer::new(
            self.vertices(),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            graphics.device,
        );
        let indices = Buffer::new(
            self.indices(),
            wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            graphics.device,
        );
        PreparedGraphic {
            vertices,
            indices,
            commands: smallvec![PreparedCommand {
                indices: 0..self
                    .indices()
                    .len()
                    .try_into()
                    .expect("too many drawn indices"),
                is_mask: false,
                binding: texture.map(TextureSource::bind_group),
            }],
        }
    }
}
