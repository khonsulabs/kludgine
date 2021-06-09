use std::sync::Arc;

use crate::{
    color::Color,
    math::{Pixels, Point, Raw, Scaled},
    scene::{Element, Target},
    text::Font,
};

#[derive(Copy, Clone, Debug)]
pub struct VMetrics {
    pub ascent: Pixels,
    pub descent: Pixels,
    pub line_gap: Pixels,
}

impl From<rusttype::VMetrics> for VMetrics {
    fn from(value: rusttype::VMetrics) -> Self {
        Self {
            ascent: Pixels::new(value.ascent),
            descent: Pixels::new(value.descent),
            line_gap: Pixels::new(value.line_gap),
        }
    }
}

impl VMetrics {
    #[must_use]
    pub fn line_height(&self) -> Pixels {
        self.height() + self.line_gap
    }

    #[must_use]
    pub fn height(&self) -> Pixels {
        self.ascent - self.descent
    }
}

#[derive(Clone, Debug)]
pub struct PreparedSpan {
    pub location: Point<f32, Raw>,
    pub data: Arc<PreparedSpanData>,
}

impl PreparedSpan {
    #[must_use]
    pub fn new(
        font: Font,
        size: Pixels,
        color: Color,
        width: Pixels,
        characters: Vec<char>,
        glyphs: Vec<GlyphInfo>,
        metrics: rusttype::VMetrics,
    ) -> Self {
        Self {
            location: Point::default(),
            data: Arc::new(PreparedSpanData {
                font,
                size,
                color,
                width,
                characters,
                glyphs,
                metrics,
            }),
        }
    }

    #[must_use]
    pub fn translate(&self, location: Point<f32, Raw>) -> Self {
        Self {
            // We want to ensure that we are pixel-aligned when rendering a span's start.
            location: location.round(),
            data: self.data.clone(),
        }
    }

    #[must_use]
    pub fn metrics(&self) -> rusttype::VMetrics {
        self.data.font.metrics(self.data.size)
    }

    pub fn render_baseline_at(
        &self,
        scene: &Target,
        location: Point<f32, Scaled>,
    ) -> crate::Result<()> {
        let effective_scale_factor = scene.scale_factor();

        let location = scene.offset_point_raw(
            (location + self.location.to_vector() / effective_scale_factor)
                * effective_scale_factor,
        );
        scene.push_element(Element::Text {
            span: self.translate(location),
            clip: scene.clip,
        });
        Ok(())
    }
}

#[derive(Debug)]
pub struct PreparedSpanData {
    pub font: Font,
    pub size: Pixels,
    pub color: Color,
    pub width: Pixels,
    pub characters: Vec<char>,
    pub glyphs: Vec<GlyphInfo>,
    pub metrics: rusttype::VMetrics,
}

#[derive(Debug)]
pub struct GlyphInfo {
    pub source_offset: usize,
    pub source: char,
    pub glyph: rusttype::PositionedGlyph<'static>,
}

impl GlyphInfo {
    #[must_use]
    pub fn width(&self) -> Pixels {
        Pixels::new(self.glyph.unpositioned().h_metrics().advance_width)
    }

    #[must_use]
    pub fn location(&self) -> Point<f32, Raw> {
        Point::new(self.glyph.position().x, self.glyph.position().y)
    }
}
