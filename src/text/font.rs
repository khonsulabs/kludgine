use std::sync::Arc;

use crate::math::Pixels;
use crossbeam::atomic::AtomicCell;
use easygpu::prelude::*;
use lazy_static::lazy_static;
use rusttype::{gpu_cache, Scale};
use stylecs::{FontStyle, Weight};

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
            handle: Arc::new(FontData { font, id }),
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

    pub fn weight(&self) -> Weight {
        match &self.handle.font {
            rusttype::Font::Ref(f) => convert_ttf_weight_to_stylecs(f.weight()),
            _ => Weight::Normal,
        }
    }

    pub fn style(&self) -> FontStyle {
        match &self.handle.font {
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

fn convert_ttf_weight_to_stylecs(weight: ttf_parser::Weight) -> stylecs::Weight {
    match weight {
        ttf_parser::Weight::Thin => stylecs::Weight::Thin,
        ttf_parser::Weight::ExtraLight => stylecs::Weight::ExtraLight,
        ttf_parser::Weight::Light => stylecs::Weight::Light,
        ttf_parser::Weight::Normal => stylecs::Weight::Normal,
        ttf_parser::Weight::Medium => stylecs::Weight::Medium,
        ttf_parser::Weight::SemiBold => stylecs::Weight::SemiBold,
        ttf_parser::Weight::Bold => stylecs::Weight::Bold,
        ttf_parser::Weight::ExtraBold => stylecs::Weight::ExtraBold,
        ttf_parser::Weight::Black => stylecs::Weight::Black,
        ttf_parser::Weight::Other(value) => stylecs::Weight::Other(value),
    }
}
// impl From<ttf_parser::Weight> for Weight {
//     fn from(weight: ttf_parser::Weight) -> Self {
// }

// impl From<Weight> for ttf_parser::Weight {
//     fn from(weight: Weight) -> Self {
//         match weight {
//             Weight::Thin => Self::Thin,
//             Weight::ExtraLight => Self::ExtraLight,
//             Weight::Light => Self::Light,
//             Weight::Normal => Self::Normal,
//             Weight::Medium => Self::Medium,
//             Weight::SemiBold => Self::SemiBold,
//             Weight::Bold => Self::Bold,
//             Weight::ExtraBold => Self::ExtraBold,
//             Weight::Black => Self::Black,
//             Weight::Other(value) => Self::Other(value),
//         }
//     }
// }
