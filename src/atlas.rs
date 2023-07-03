use std::fmt::Debug;
use std::ops::{Add, Div, Neg};
use std::sync::{Arc, PoisonError, RwLock};

use alot::{LotId, Lots};
use guillotiere::{AllocId, AtlasAllocator};

use crate::math::{Point, Rect, Size, ToFloat, UPixels};
use crate::pipeline::{PreparedGraphic, Vertex};
use crate::{sealed, Graphics, Texture, TextureSource, WgpuDeviceAndQueue};

#[derive(Clone)]
pub struct TextureCollection {
    data: Arc<RwLock<Data>>,
}

struct Data {
    rects: AtlasAllocator,
    texture: Texture,
    textures: Lots<AllocatedTexture>,
}

struct AllocatedTexture {
    id: AllocId,
    rect: Rect<UPixels>,
}

impl TextureCollection {
    pub(crate) fn new_generic(
        initial_size: Size<UPixels>,
        format: wgpu::TextureFormat,
        graphics: &impl WgpuDeviceAndQueue,
    ) -> Self {
        let texture = Texture::new_generic(
            graphics,
            initial_size,
            format,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );

        Self {
            data: Arc::new(RwLock::new(Data {
                rects: AtlasAllocator::new(guillotiere::size2(
                    initial_size.width.0.try_into().expect("width too large"),
                    initial_size.height.0.try_into().expect("height too large"),
                )),
                texture,
                textures: Lots::new(),
            })),
        }
    }

    #[must_use]
    pub fn new(
        initial_size: Size<UPixels>,
        format: wgpu::TextureFormat,
        graphics: &Graphics<'_>,
    ) -> Self {
        Self::new_generic(initial_size, format, graphics)
    }

    pub fn push_texture(
        &mut self,
        data: &[u8],
        data_layout: wgpu::ImageDataLayout,
        size: Size<UPixels>,
        graphics: &wgpu::Queue,
    ) -> CollectedTexture {
        let mut this = self
            .data
            .write()
            .map_or_else(PoisonError::into_inner, |g| g);
        let allocation = this
            .rects
            .allocate(guillotiere::size2(
                size.width.0.try_into().expect("width too large"),
                size.height.0.try_into().expect("height too large"),
            ))
            .expect("TODO: implement growth");

        let p1: Point<UPixels> = Point {
            x: u32::try_from(allocation.rectangle.min.x)
                .expect("invalid allocation")
                .into(),
            y: u32::try_from(allocation.rectangle.min.y)
                .expect("invalid allocation")
                .into(),
        };

        let p2: Point<UPixels> = Point {
            x: u32::try_from(allocation.rectangle.max.x)
                .expect("invalid allocation")
                .into(),
            y: u32::try_from(allocation.rectangle.max.y)
                .expect("invalid allocation")
                .into(),
        };
        let rect = Rect::from_extents(p1, p2);

        graphics.write_texture(
            wgpu::ImageCopyTexture {
                texture: &this.texture.wgpu,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: rect.origin.x.0,
                    y: rect.origin.y.0,
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
            id: Arc::new(this.textures.push(AllocatedTexture {
                id: allocation.id,
                rect,
            })),
            region: Rect::from_extents(p1, p2),
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
            graphics.queue,
        )
    }

    pub fn size(&self) -> Size<UPixels> {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.texture.size()
    }

    fn free(&mut self, id: LotId) {
        let mut data = self
            .data
            .write()
            .map_or_else(PoisonError::into_inner, |g| g);
        let allocation = data.textures.remove(id).expect("invalid texture free");
        data.rects.deallocate(allocation.id);
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

    fn is_mask(&self) -> bool {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.texture.is_mask()
    }
}

#[derive(Clone)]
pub struct CollectedTexture {
    collection: TextureCollection,
    id: Arc<LotId>,
    pub(crate) region: Rect<UPixels>,
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

    fn is_mask(&self) -> bool {
        self.collection.is_mask()
    }
}
