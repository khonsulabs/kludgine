use std::{
    convert::TryFrom,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use image::{DynamicImage, RgbaImage};
use lazy_static::lazy_static;
use winit::window::Icon;

use crate::math::Size;

lazy_static! {
    static ref GLOBAL_ID_CELL: AtomicU64 = AtomicU64::new(0);
}

/// Embeds a texture in the binary.
#[macro_export]
macro_rules! include_texture {
    ($image_path:expr) => {{
        let image_bytes = std::include_bytes!($image_path);
        <$crate::texture::Texture as std::convert::TryFrom<&[u8]>>::try_from(image_bytes)
    }};
}

/// An image that can be used as a sprite. Cheap to clone.
#[derive(Debug, Clone)]
pub struct Texture {
    id: u64,
    /// The image behind the texture.
    pub image: Arc<RgbaImage>,
}

impl Texture {
    /// The unique ID of this texture. This depends on load order and is not
    /// related to the image data in any way.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Creates a new texture from an image.
    #[must_use]
    pub fn new(image: &DynamicImage) -> Self {
        let image = image.to_rgba8();
        let id = GLOBAL_ID_CELL.fetch_add(1, Ordering::SeqCst);
        Self {
            id,
            image: Arc::new(image),
        }
    }

    /// Loads a texture from an image at `path`
    pub fn load<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let img = image::open(path)?;

        Ok(Self::new(&img))
    }

    /// Returns the size of the image.
    #[must_use]
    pub fn size(&self) -> Size<u32> {
        let (w, h) = self.image.dimensions();
        Size::new(w as u32, h as u32)
    }

    /// Returns the raw image data.
    #[must_use]
    pub fn rgba_pixels(&self) -> Vec<u8> {
        (*self.image).clone().into_vec()
    }

    /// Converts the underlying image into a format compatible with `winit` for
    /// use as a window icon.
    pub fn window_icon(&self) -> Result<Icon, winit::window::BadIcon> {
        Icon::from_rgba(self.rgba_pixels(), self.image.width(), self.image.height())
    }
}

impl<'a> TryFrom<&'a [u8]> for Texture {
    type Error = crate::Error;

    fn try_from(bytes: &[u8]) -> crate::Result<Self> {
        let img = image::load_from_memory(bytes)?;

        Ok(Self::new(&img))
    }
}
