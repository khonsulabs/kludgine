use super::{math::Size, KludgineHandle, KludgineResult};
use crossbeam::atomic::AtomicCell;
use image::{DynamicImage, RgbaImage};
use lazy_static::lazy_static;
use rgx::core::*;
use std::path::Path;

lazy_static! {
    static ref GLOBAL_ID_CELL: AtomicCell<u64> = AtomicCell::new(0);
}

#[macro_export]
macro_rules! include_texture {
    ($image_path:expr) => {{
        let image_bytes = std::include_bytes!($image_path);
        Texture::from_bytes(image_bytes)
    }};
}

#[derive(Debug, Clone)]
pub struct Texture {
    pub(crate) handle: KludgineHandle<TextureData>,
}

#[derive(Debug)]
pub(crate) struct TextureData {
    pub id: u64,
    pub image: RgbaImage,
}

impl Texture {
    pub fn new(image: DynamicImage) -> Self {
        let image = image.to_rgba();
        let id = GLOBAL_ID_CELL.fetch_add(1);
        Self {
            handle: KludgineHandle::new(TextureData { id, image }),
        }
    }

    pub fn load<P: AsRef<Path>>(from_path: P) -> KludgineResult<Self> {
        let img = image::open(from_path)?;

        Ok(Self::new(img))
    }

    pub fn from_bytes(bytes: &[u8]) -> KludgineResult<Self> {
        let img = image::load_from_memory(bytes)?;

        Ok(Self::new(img))
    }

    pub async fn size(&self) -> Size<u32> {
        let texture = self.handle.read().await;
        let (w, h) = texture.image.dimensions();
        Size::new(w as u32, h as u32)
    }
}

#[derive(Clone)]
pub struct LoadedTexture {
    pub(crate) handle: KludgineHandle<LoadedTextureData>,
}

pub(crate) struct LoadedTextureData {
    pub texture: Texture,
    pub binding: Option<BindingGroup>,
}

impl LoadedTexture {
    pub fn new(texture: &Texture) -> Self {
        LoadedTexture {
            handle: KludgineHandle::new(LoadedTextureData {
                texture: texture.clone(),
                binding: None,
            }),
        }
    }
}
