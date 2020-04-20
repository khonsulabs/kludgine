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
pub(crate) struct Text {
    pub handle: KludgineHandle<TextData>,
}

impl Text {
    pub fn new(
        font: Font,
        size: f32,
        text: String,
        location: Point,
        max_width: Option<f32>,
    ) -> Self {
        Self {
            handle: KludgineHandle::new(TextData {
                font,
                text,
                size,
                location,
                max_width,
                positioned_glyphs: None,
            }),
        }
    }
}

pub struct TextData {
    pub font: Font,
    pub size: f32,
    pub text: String,
    pub location: Point,
    pub max_width: Option<f32>,
    pub positioned_glyphs: Option<Vec<rusttype::PositionedGlyph<'static>>>,
}
