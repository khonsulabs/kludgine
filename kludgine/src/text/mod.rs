use super::{math::Point, KludgineHandle};
use crossbeam::atomic::AtomicCell;
use lazy_static::lazy_static;
use rgx::core::*;
use rusttype::gpu_cache;

#[cfg(feature = "bundled-fonts-enabled")]
pub mod bundled_fonts;

lazy_static! {
    static ref GLOBAL_ID_CELL: AtomicCell<u64> = { AtomicCell::new(0) };
}

/// Font provides TrueType Font rendering
#[derive(Clone)]
pub struct Font {
    pub(crate) handle: KludgineHandle<FontData>,
}

impl Font {
    pub fn try_from_bytes(bytes: &'static [u8]) -> Option<Font> {
        let font = rusttype::Font::try_from_bytes(bytes)?;
        Some(Font {
            handle: KludgineHandle::new(FontData {
                font,
                id: GLOBAL_ID_CELL.fetch_add(1),
            }),
        })
    }

    pub fn metrics(&self, size: f32) -> rusttype::VMetrics {
        let font = self.handle.read().expect("Error reading font");
        font.font.v_metrics(rusttype::Scale::uniform(size))
    }

    pub fn family(&self) -> Option<String> {
        let font = self.handle.read().expect("Error reading font");
        match &font.font {
            rusttype::Font::Ref(f) => f.family_name(),
            _ => None,
        }
    }

    pub fn weight(&self) -> ttf_parser::Weight {
        let font = self.handle.read().expect("Error reading font");
        match &font.font {
            rusttype::Font::Ref(f) => f.weight(),
            _ => ttf_parser::Weight::Normal,
        }
    }
}

pub(crate) struct FontData {
    pub(crate) id: u64,
    pub(crate) font: rusttype::Font<'static>,
}

#[derive(Clone)]
pub(crate) struct LoadedFont {
    pub handle: KludgineHandle<LoadedFontData>,
}

impl LoadedFont {
    pub fn new(font: &Font) -> Self {
        Self {
            handle: KludgineHandle::new(LoadedFontData {
                font: font.clone(),
                cache: gpu_cache::Cache::builder().dimensions(512, 512).build(),
                binding: None,
                texture: None,
            }),
        }
    }
}

pub(crate) struct LoadedFontData {
    pub font: Font,
    pub cache: gpu_cache::Cache<'static>,
    pub(crate) binding: Option<BindingGroup>,
    pub(crate) texture: Option<rgx::core::Texture>,
}

#[derive(Clone)]
pub(crate) struct RenderedSpan {
    pub handle: KludgineHandle<SpanData>,
}

impl RenderedSpan {
    pub fn new(
        text: String,
        font: Font,
        size: f32,
        color: Rgba,
        location: Point,
        max_width: Option<f32>,
    ) -> Self {
        Self {
            handle: KludgineHandle::new(SpanData {
                font,
                text,
                size,
                color,
                location,
                max_width,
                positioned_glyphs: None,
            }),
        }
    }
}

pub struct SpanData {
    pub font: Font,
    pub size: f32,
    pub text: String,
    pub color: Rgba,
    pub location: Point,
    pub max_width: Option<f32>,
    pub positioned_glyphs: Option<Vec<rusttype::PositionedGlyph<'static>>>,
}
