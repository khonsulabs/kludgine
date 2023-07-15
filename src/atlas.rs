use std::fmt::Debug;
use std::ops::Div;
use std::sync::{Arc, PoisonError, RwLock};

use alot::{LotId, Lots};
use figures::units::UPx;
use figures::{Rect, Size};
use shelf_packer::{Allocation, ShelfPacker};

use crate::pipeline::{PreparedGraphic, Vertex};
use crate::{sealed, Graphics, Texture, TextureSource, WgpuDeviceAndQueue};

/// A collection of multiple textures, managed as a single texture on the GPU.
/// This type is often called an atlas.
///
/// The collection is currently fixed-size and will panic when an allocation
/// fails. In the future, this type will dynamically grow as more textures are
/// added to it.
///
/// In general, this type should primarly be used with similarly-sized graphics,
/// otherwise the packing may be inefficient. For example, packing many images
/// that are multiples of 32px wide/tall will be very efficient. Interally, this
/// type is used for caching rendered glyphs on the GPU.
#[derive(Clone)]
pub struct TextureCollection {
    format: wgpu::TextureFormat,
    data: Arc<RwLock<Data>>,
}

struct Data {
    rects: ShelfPacker,
    texture: Texture,
    textures: Lots<Allocation>,
}

impl TextureCollection {
    pub(crate) fn new_generic(
        initial_size: Size<UPx>,
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
            format,
            data: Arc::new(RwLock::new(Data {
                rects: ShelfPacker::new(
                    initial_size,
                    initial_size.width.0.try_into().expect("width too large"),
                ),
                texture,
                textures: Lots::new(),
            })),
        }
    }

    /// Returns a new atlas of the given size and format.
    #[must_use]
    pub fn new(
        initial_size: Size<UPx>,
        format: wgpu::TextureFormat,
        graphics: &Graphics<'_>,
    ) -> Self {
        Self::new_generic(initial_size, format, graphics)
    }

    /// Pushes image data to a specific region of the texture.
    ///
    /// The data format must match the format of the texture, and must be sized
    /// exactly according to the `data_layout` and `size` and format.
    ///
    /// The returned [`CollectedTexture`] will automatically free the space it
    /// occupies when the last instance is dropped.
    pub fn push_texture(
        &mut self,
        data: &[u8],
        data_layout: wgpu::ImageDataLayout,
        size: Size<UPx>,
        graphics: &wgpu::Queue,
    ) -> CollectedTexture {
        let mut this = self
            .data
            .write()
            .map_or_else(PoisonError::into_inner, |g| g);
        let allocation = this.rects.allocate(size).expect("TODO: implement growth");

        graphics.write_texture(
            wgpu::ImageCopyTexture {
                texture: &this.texture.wgpu,
                mip_level: 0,
                origin: allocation.rect.origin.into(),
                aspect: wgpu::TextureAspect::All,
            },
            data,
            data_layout,
            size.into(),
        );
        CollectedTexture {
            collection: self.clone(),
            id: Arc::new(this.textures.push(allocation)),
            region: allocation.rect,
        }
    }

    /// Pushes an image to this collection.
    ///
    /// The returned [`CollectedTexture`] will automatically free the space it
    /// occupies when the last instance is dropped.
    ///
    /// # Panics
    ///
    /// Currently this only supports uploading to Rgba8 formatted textures.
    #[cfg(feature = "image")]
    pub fn push_image(
        &mut self,
        image: &image::DynamicImage,
        graphics: &Graphics<'_>,
    ) -> CollectedTexture {
        assert!(matches!(
            self.format,
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb
        ));
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

    /// Returns the current size of the underlying texture.
    pub fn size(&self) -> Size<UPx> {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.texture.size()
    }

    fn free(&mut self, id: LotId) {
        let mut data = self
            .data
            .write()
            .map_or_else(PoisonError::into_inner, |g| g);
        let allocation = data.textures.remove(id).expect("invalid texture free");
        data.rects.free(allocation.id);
    }

    fn prepare<Unit>(
        &self,
        id: LotId,
        dest: Rect<Unit>,
        graphics: &Graphics<'_>,
    ) -> PreparedGraphic<Unit>
    where
        Unit: figures::Unit + Div<i32, Output = Unit>,
        Vertex<Unit>: bytemuck::Pod,
    {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        let texture = &data.textures[id];
        data.texture.prepare_partial(texture.rect, dest, graphics)
    }

    /// Returns a [`PreparedGraphic`] for the entire texture.
    ///
    /// This is primarily a debugging tool, as generally the
    /// [`CollectedTexture`]s are rendered instead.
    pub fn prepare_entire_colection<Unit>(
        &self,
        dest: Rect<Unit>,
        graphics: &Graphics<'_>,
    ) -> PreparedGraphic<Unit>
    where
        Unit: figures::Unit,
        Vertex<Unit>: bytemuck::Pod,
    {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.texture.prepare(dest, graphics)
    }

    /// Returns the format of the texture backing this collection.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
}

impl TextureSource for TextureCollection {}

impl sealed::TextureSource for TextureCollection {
    fn bind_group(&self) -> Arc<wgpu::BindGroup> {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.texture.bind_group()
    }

    fn id(&self) -> sealed::TextureId {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.texture.id()
    }

    fn is_mask(&self) -> bool {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.texture.is_mask()
    }

    fn default_rect(&self) -> Rect<UPx> {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.texture.default_rect()
    }
}

/// A texture that is contained within a [`TextureCollection`].
#[derive(Clone)]
pub struct CollectedTexture {
    collection: TextureCollection,
    id: Arc<LotId>,
    pub(crate) region: Rect<UPx>,
}

impl Debug for CollectedTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollectedTexture")
            .field("id", &self.id)
            .field("region", &self.region)
            .finish()
    }
}

impl CollectedTexture {
    /// Returns a [`PreparedGraphic`] that renders this texture at `dest`.
    pub fn prepare<Unit>(&self, dest: Rect<Unit>, graphics: &Graphics<'_>) -> PreparedGraphic<Unit>
    where
        Unit: figures::Unit + Div<i32, Output = Unit>,
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
    fn bind_group(&self) -> Arc<wgpu::BindGroup> {
        self.collection.bind_group()
    }

    fn id(&self) -> sealed::TextureId {
        self.collection.id()
    }

    fn is_mask(&self) -> bool {
        self.collection.is_mask()
    }

    fn default_rect(&self) -> Rect<UPx> {
        self.region
    }
}
