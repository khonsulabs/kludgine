use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use easygpu::prelude::*;
use figures::Figure;
use lazy_static::lazy_static;
use rusttype::{gpu_cache, Scale};

use crate::math::Pixels;

lazy_static! {
    static ref GLOBAL_ID_CELL: AtomicU64 = AtomicU64::new(0);
}

/// Embeds a font into your executable.
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
    /// Try to load a font from `bytes`.
    #[must_use]
    pub fn try_from_bytes(bytes: &'static [u8]) -> Option<Self> {
        let font = rusttype::Font::try_from_bytes(bytes)?;
        let id = GLOBAL_ID_CELL.fetch_add(1, Ordering::SeqCst);
        Some(Self {
            handle: Arc::new(FontData { id, font }),
        })
    }

    /// Returns the unique ID of the font. This ID depends on load order, and is
    /// not from the font data.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.handle.id
    }

    /// Measures the vertical metrics for a given size.
    #[must_use]
    pub fn metrics(&self, size: Figure<f32, Pixels>) -> rusttype::VMetrics {
        self.handle
            .font
            .v_metrics(rusttype::Scale::uniform(size.get()))
    }

    /// Returns the name of the font's family, if available.
    #[must_use]
    pub fn family(&self) -> Option<String> {
        match &self.handle.font {
            rusttype::Font::Ref(f) => f.family_name(),
            rusttype::Font::Owned(_) => None,
        }
    }

    #[must_use]
    pub(crate) fn glyph(&self, c: char) -> rusttype::Glyph<'static> {
        self.handle.font.glyph(c)
    }

    #[must_use]
    pub(crate) fn pair_kerning(
        &self,
        size: f32,
        a: rusttype::GlyphId,
        b: rusttype::GlyphId,
    ) -> f32 {
        self.handle.font.pair_kerning(Scale::uniform(size), a, b)
    }
}

#[derive(Debug)]
pub struct FontData {
    pub(crate) id: u64,
    pub(crate) font: rusttype::Font<'static>,
}

pub struct LoadedFont {
    pub font: Font,
    pub cache: gpu_cache::Cache<'static>,
    pub(crate) binding: Option<BindingGroup>,
    pub(crate) texture: Option<Texture>,
}

impl Debug for LoadedFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedFont")
            .field("font", &self.font)
            .field("binding", &self.binding)
            .field("texture", &self.texture)
            .finish()
    }
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
