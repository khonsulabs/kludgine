#[cfg(feature = "bundled-fonts-enabled")]
pub mod bundled_fonts;
pub(crate) mod font;
/// Types for handling perpared text.
pub mod prepared;
use figures::Figure;
pub use font::Font;
use rusttype::Scale;

use self::prepared::{GlyphInfo, PreparedSpan};
use crate::{
    color::Color,
    math::{Pixels, Scaled},
    prelude::Target,
};

/// Text rendering functionality
pub enum Text {}

impl Text {
    /// Prepares `text` to be rendered with the provided settings.
    #[must_use]
    pub fn prepare(
        text: &str,
        font: &Font,
        size: Figure<f32, Scaled>,
        color: Color,
        scene: &Target,
    ) -> PreparedSpan {
        let size_in_pixels = size * scene.scale_factor();
        let characters = text.chars().collect::<Vec<_>>();
        let mut caret = Pixels::new(0.);
        let mut glyphs = Vec::new();
        let mut last_glyph_id = None;
        for (source_offset, &c) in characters.iter().enumerate() {
            let base_glyph = font.glyph(c);
            if let Some(id) = last_glyph_id.take() {
                caret += Pixels::new(font.pair_kerning(size_in_pixels.get(), id, base_glyph.id()));
            }
            last_glyph_id = Some(base_glyph.id());
            let glyph = base_glyph
                .scaled(Scale::uniform(size_in_pixels.get()))
                .positioned(rusttype::point(caret.get(), 0.0));

            caret += Pixels::new(glyph.unpositioned().h_metrics().advance_width);
            glyphs.push(GlyphInfo {
                source_offset,
                source: c,
                glyph,
            });
        }

        PreparedSpan::new(
            font.clone(),
            size_in_pixels,
            color,
            caret,
            characters,
            glyphs,
            font.metrics(size_in_pixels),
        )
    }
}
