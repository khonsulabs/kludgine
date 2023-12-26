use std::fmt::Debug;
use std::ops::Div;
use std::sync::{Arc, PoisonError, RwLock};

use alot::{LotId, Lots};
use etagere::{Allocation, BucketedAtlasAllocator};
use figures::units::UPx;
use figures::{IntoSigned, IntoUnsigned, Point, Px2D, Rect, Size, UPx2D};

use crate::pipeline::{PreparedGraphic, Vertex};
use crate::{sealed, CanRenderTo, Graphics, Kludgine, KludgineGraphics, Texture, TextureSource};

fn atlas_usages() -> wgpu::TextureUsages {
    wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_DST
        | wgpu::TextureUsages::COPY_SRC
}

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
    filter_mode: wgpu::FilterMode,
    data: Arc<RwLock<Data>>,
}

struct Data {
    rects: BucketedAtlasAllocator,
    texture: Texture,
    textures: Lots<Allocation>,
}

impl TextureCollection {
    pub(crate) fn new_generic(
        initial_size: Size<UPx>,
        format: wgpu::TextureFormat,
        filter_mode: wgpu::FilterMode,
        graphics: &impl KludgineGraphics,
    ) -> Self {
        let texture =
            Texture::new_generic(graphics, initial_size, format, atlas_usages(), filter_mode);

        let initial_size = initial_size.into_signed();
        Self {
            format,
            filter_mode,
            data: Arc::new(RwLock::new(Data {
                rects: BucketedAtlasAllocator::new(etagere::euclid::Size2D::new(
                    initial_size.width.into(),
                    initial_size.height.into(),
                )),
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
        filter_mode: wgpu::FilterMode,
        graphics: &Graphics<'_>,
    ) -> Self {
        Self::new_generic(initial_size, format, filter_mode, graphics)
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
        graphics: &Graphics<'_>,
    ) -> CollectedTexture {
        self.push_texture_generic(data, data_layout, size, graphics)
    }

    pub(crate) fn push_texture_generic(
        &mut self,
        data: &[u8],
        data_layout: wgpu::ImageDataLayout,
        size: Size<UPx>,
        graphics: &impl KludgineGraphics,
    ) -> CollectedTexture {
        let mut this = self
            .data
            .write()
            .map_or_else(PoisonError::into_inner, |g| g);
        let signed_size = size.into_signed();
        let allocation = loop {
            if let Some(allocation) = this.rects.allocate(etagere::euclid::Size2D::new(
                signed_size.width.into(),
                signed_size.height.into(),
            )) {
                break allocation;
            }

            let new_size = this.texture.size * 2;
            let new_texture = Texture::new_generic(
                graphics,
                new_size,
                self.format,
                atlas_usages(),
                self.filter_mode,
            );
            let mut commands = graphics
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            commands.copy_texture_to_texture(
                this.texture.data.wgpu.as_image_copy(),
                new_texture.data.wgpu.as_image_copy(),
                this.texture.size.into(),
            );
            graphics.queue().submit([commands.finish()]);

            this.rects.grow(etagere::euclid::Size2D::new(
                new_size.width.into_signed().get(),
                new_size.height.into_signed().get(),
            ));
            this.texture = new_texture;
        };

        let region = Rect::new(
            Point::px(allocation.rectangle.min.x, allocation.rectangle.min.y).into_unsigned(),
            size,
        );

        graphics.queue().write_texture(
            wgpu::ImageCopyTexture {
                texture: &this.texture.data.wgpu,
                mip_level: 0,
                origin: region.origin.into(),
                aspect: wgpu::TextureAspect::All,
            },
            data,
            data_layout,
            size.into(),
        );
        CollectedTexture {
            collection: self.clone(),
            id: Arc::new(this.textures.push(allocation)),
            region,
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
            Size::upx(image.width(), image.height()),
            graphics,
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
        data.rects.deallocate(allocation.id);
    }

    fn prepare<Unit>(
        &self,
        src: Rect<UPx>,
        dest: Rect<Unit>,
        graphics: &Graphics<'_>,
    ) -> PreparedGraphic<Unit>
    where
        Unit: figures::Unit + Div<i32, Output = Unit>,
        Vertex<Unit>: bytemuck::Pod,
    {
        let data = self.data.read().map_or_else(PoisonError::into_inner, |g| g);
        data.texture.prepare_partial(src, dest, graphics)
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

impl CanRenderTo for TextureCollection {
    fn can_render_to(&self, kludgine: &Kludgine) -> bool {
        self.data
            .read()
            .map_or_else(PoisonError::into_inner, |g| g)
            .texture
            .can_render_to(kludgine)
    }
}

impl TextureSource for TextureCollection {}

impl sealed::TextureSource for TextureCollection {
    fn bind_group(&self, graphics: &impl sealed::KludgineGraphics) -> Arc<wgpu::BindGroup> {
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
            .finish_non_exhaustive()
    }
}

impl CollectedTexture {
    /// Returns a [`PreparedGraphic`] that renders this texture at `dest`.
    pub fn prepare<Unit>(&self, dest: Rect<Unit>, graphics: &Graphics<'_>) -> PreparedGraphic<Unit>
    where
        Unit: figures::Unit + Div<i32, Output = Unit>,
        Vertex<Unit>: bytemuck::Pod,
    {
        self.collection.prepare(self.region, dest, graphics)
    }
}

impl Drop for CollectedTexture {
    fn drop(&mut self) {
        if Arc::strong_count(&self.id) == 1 {
            self.collection.free(*self.id);
        }
    }
}

impl CanRenderTo for CollectedTexture {
    fn can_render_to(&self, kludgine: &Kludgine) -> bool {
        self.collection.can_render_to(kludgine)
    }
}

impl TextureSource for CollectedTexture {}

impl sealed::TextureSource for CollectedTexture {
    fn bind_group(&self, graphics: &impl sealed::KludgineGraphics) -> Arc<wgpu::BindGroup> {
        self.collection.bind_group(graphics)
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
