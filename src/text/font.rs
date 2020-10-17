use crate::{math::Pixels, style::Weight, Handle};
use crossbeam::atomic::AtomicCell;
use easygpu::prelude::*;
use lazy_static::lazy_static;
use rusttype::{gpu_cache, Scale};

lazy_static! {
    static ref GLOBAL_ID_CELL: AtomicCell<u64> = AtomicCell::new(0);
}

#[macro_export]
macro_rules! include_font {
    ($path:expr) => {{
        let bytes = std::include_bytes!($path);
        Font::try_from_bytes(bytes as &[u8]).expect("Error loading bundled font")
    }};
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum FontStyle {
    Regular,
    Italic,
    Oblique,
}

impl Default for FontStyle {
    fn default() -> Self {
        FontStyle::Regular
    }
}

/// Font provides TrueType Font rendering
#[derive(Clone, Debug)]
pub struct Font {
    pub(crate) id: u64,
    pub(crate) handle: Handle<FontData>,
}

impl Font {
    pub fn try_from_bytes(bytes: &'static [u8]) -> Option<Font> {
        let font = rusttype::Font::try_from_bytes(bytes)?;
        let id = GLOBAL_ID_CELL.fetch_add(1);
        Some(Font {
            id,
            handle: Handle::new(FontData { font, id }),
        })
    }

    pub async fn id(&self) -> u64 {
        let font = self.handle.read().await;
        font.id
    }

    pub async fn metrics(&self, size: Pixels) -> rusttype::VMetrics {
        let font = self.handle.read().await;
        font.font.v_metrics(rusttype::Scale::uniform(size.get()))
    }

    pub async fn family(&self) -> Option<String> {
        let font = self.handle.read().await;
        match &font.font {
            rusttype::Font::Ref(f) => f.family_name(),
            _ => None,
        }
    }

    pub async fn weight(&self) -> Weight {
        let font = self.handle.read().await;
        match &font.font {
            rusttype::Font::Ref(f) => f.weight().into(),
            _ => Weight::Normal,
        }
    }

    pub async fn style(&self) -> FontStyle {
        let font = self.handle.read().await;

        match &font.font {
            rusttype::Font::Ref(f) => {
                if f.is_italic() {
                    FontStyle::Italic
                } else if f.is_oblique() {
                    FontStyle::Oblique
                } else {
                    FontStyle::Regular
                }
            }
            _ => FontStyle::Regular,
        }
    }

    pub async fn glyph(&self, c: char) -> rusttype::Glyph<'static> {
        let font = self.handle.read().await;
        font.font.glyph(c)
    }

    pub async fn pair_kerning(&self, size: f32, a: rusttype::GlyphId, b: rusttype::GlyphId) -> f32 {
        let font = self.handle.read().await;
        font.font.pair_kerning(Scale::uniform(size), a, b)
    }
}

#[derive(Debug)]
pub(crate) struct FontData {
    pub(crate) id: u64,
    pub(crate) font: rusttype::Font<'static>,
}

pub(crate) struct LoadedFont {
    pub font: Font,
    pub cache: gpu_cache::Cache<'static>,
    pub(crate) binding: Option<BindingGroup>,
    pub(crate) texture: Option<Texture>,
}

impl LoadedFont {
    pub fn new(font: &Font) -> Self {
        Self {
            font: font.clone(),
            cache: gpu_cache::Cache::builder().dimensions(512, 512).build(),
            binding: None,
            texture: None,
        }
    }
}
