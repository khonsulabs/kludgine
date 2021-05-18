use std::sync::Arc;

use crossbeam::atomic::AtomicCell;
use easygpu::prelude::*;
use lazy_static::lazy_static;
use rusttype::{gpu_cache, Scale};

use crate::math::Pixels;

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

/// Font provides TrueType Font rendering
#[derive(Clone, Debug)]
pub struct Font {
    pub(crate) handle: Arc<FontData>,
}

impl Font {
    pub fn try_from_bytes(bytes: &'static [u8]) -> Option<Font> {
        let font = rusttype::Font::try_from_bytes(bytes)?;
        let id = GLOBAL_ID_CELL.fetch_add(1);
        Some(Font {
            handle: Arc::new(FontData { id, font }),
        })
    }

    pub fn id(&self) -> u64 {
        self.handle.id
    }

    pub fn metrics(&self, size: Pixels) -> rusttype::VMetrics {
        self.handle
            .font
            .v_metrics(rusttype::Scale::uniform(size.get()))
    }

    pub fn family(&self) -> Option<String> {
        match &self.handle.font {
            rusttype::Font::Ref(f) => f.family_name(),
            _ => None,
        }
    }

    pub fn glyph(&self, c: char) -> rusttype::Glyph<'static> {
        self.handle.font.glyph(c)
    }

    pub fn pair_kerning(&self, size: f32, a: rusttype::GlyphId, b: rusttype::GlyphId) -> f32 {
        self.handle.font.pair_kerning(Scale::uniform(size), a, b)
    }
}

#[derive(Debug)]
pub(crate) struct FontData {
    pub(crate) id: u64,
    pub(crate) font: rusttype::Font<'static>,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct LoadedFont {
    pub font: Font,
    #[derivative(Debug = "ignore")]
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
