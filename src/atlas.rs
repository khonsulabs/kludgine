use std::fmt::Debug;
use std::ops::{Add, Div, Neg};
use std::sync::{Arc, PoisonError, RwLock};

use alot::{LotId, Lots};

use crate::math::{Rect, Size, ToFloat, UPixels};
use crate::pack::{TextureAllocation, TexturePacker};
use crate::sealed;
use crate::shapes::{PreparedGraphic, Vertex};
use crate::{Graphics, Texture, TextureSource, WgpuDeviceAndQueue};

#[derive(Debug, Clone)]
pub struct TextureCollection {
    data: Arc<RwLock<Data>>,
}

#[derive(Debug)]
struct Data {
    rects: TexturePacker,
    texture: Texture,
    textures: Lots<TextureAllocation>,
}

impl TextureCollection {
    pub fn new(
        initial_size: Size<UPixels>,
        minimum_column_width: u16,
        format: wgpu::TextureFormat,
        graphics: &impl WgpuDeviceAndQueue,
    ) -> Self {
        let texture = Texture::new(
            graphics,
            initial_size,
            format,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );

        Self {
            data: Arc::new(RwLock::new(Data {
                rects: TexturePacker::new(initial_size, minimum_column_width),
                texture,
                textures: Lots::new(),
            })),
        }
    }

    pub fn push_texture(
        &mut self,
        data: &[u8],
        data_layout: wgpu::ImageDataLayout,
        size: Size<UPixels>,
        graphics: &Graphics<'_>,
    ) -> CollectedTexture {
        let mut this = self
            .data
            .write()
            .map_or_else(PoisonError::into_inner, |g| g);
        let allocation = this.rects.allocate(size).expect("TODO: implement growth");
        graphics.queue().write_texture(
            wgpu::ImageCopyTexture {
                texture: &this.texture.wgpu,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: allocation.rect.origin.x.into(),
                    y: allocation.rect.origin.y.into(),
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            data_layout,
            size.into(),
        );
        CollectedTexture {
            collection: self.clone(),
            id: Arc::new(this.textures.push(allocation)),
        }
    }

    #[cfg(feature = "image")]
    pub fn push_image(
        &mut self,
        image: &image::DynamicImage,
        graphics: &Graphics<'_>,
    ) -> CollectedTexture {
        // TODO this isn't correct for all texture formats, but there's limited
        // conversion format support for the image crate. We will have to create
        // our own conversion formats for other texture formats, or we could add
        // a generic paramter to TextureCollection to determine its texture
        // format, allowing this function to only be present on types that we
        // can convert to using the image crate.
        let image = image.to_rgba8();
        self.push_texture(
            image.as_raw(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(image.width() * 4),
                rows_per_image: None,
            },
            Size::new(image.width(), image.height()),
            graphics,
        )
    }

    pub fn size(&self) -> Size<UPixels> {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.texture.size()
    }

    pub fn pixels_allocated(&self) -> UPixels {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.rects.allocated()
    }

    fn free(&mut self, id: LotId) {
        let mut data = self
            .data
            .write()
            .map_or_else(PoisonError::into_inner, |g| g);
        let allocation = data.textures.remove(id).expect("invalid texture free");
        data.rects.free(allocation.allocation);
    }

    fn prepare<Unit>(
        &self,
        id: LotId,
        dest: Rect<Unit>,
        graphics: &Graphics<'_>,
    ) -> PreparedGraphic<Unit>
    where
        Unit: Add<Output = Unit>
            + ToFloat<Float = f32>
            + Div<i32, Output = Unit>
            + Neg<Output = Unit>
            + From<i32>
            + Ord
            + Copy
            + Debug,
        Vertex<Unit>: bytemuck::Pod,
    {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        let texture = &data.textures[id];
        data.texture.prepare_partial(texture.rect, dest, graphics)
    }

    pub fn prepare_entire_colection<Unit>(
        &self,
        dest: Rect<Unit>,
        graphics: &Graphics<'_>,
    ) -> PreparedGraphic<Unit>
    where
        Unit: Add<Output = Unit>
            + ToFloat<Float = f32>
            + Div<i32, Output = Unit>
            + Neg<Output = Unit>
            + From<i32>
            + Ord
            + Copy
            + Debug,
        Vertex<Unit>: bytemuck::Pod,
    {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.texture.prepare(dest, graphics)
    }
}

impl TextureSource for TextureCollection {}

impl sealed::TextureSource for TextureCollection {
    fn bind_group(&self, graphics: &Graphics<'_>) -> Arc<wgpu::BindGroup> {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.texture.bind_group(graphics)
    }

    fn id(&self) -> sealed::TextureId {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.texture.id()
    }
}

#[derive(Clone, Debug)]
pub struct CollectedTexture {
    collection: TextureCollection,
    id: Arc<LotId>,
}

impl CollectedTexture {
    pub fn prepare<Unit>(&self, dest: Rect<Unit>, graphics: &Graphics<'_>) -> PreparedGraphic<Unit>
    where
        Unit: Add<Output = Unit>
            + ToFloat<Float = f32>
            + Div<i32, Output = Unit>
            + Neg<Output = Unit>
            + From<i32>
            + Ord
            + Copy
            + Debug,
        Vertex<Unit>: bytemuck::Pod,
    {
        self.collection.prepare(*self.id, dest, graphics)
    }
}

impl Drop for CollectedTexture {
    fn drop(&mut self) {
        if Arc::strong_count(&self.id) == 1 {
            self.collection.free(*self.id);
        }
    }
}

impl TextureSource for CollectedTexture {}

impl sealed::TextureSource for CollectedTexture {
    fn bind_group(&self, graphics: &Graphics<'_>) -> Arc<wgpu::BindGroup> {
        self.collection.bind_group(graphics)
    }

    fn id(&self) -> sealed::TextureId {
        self.collection.id()
    }
}
