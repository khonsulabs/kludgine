use std::{ops::Deref, sync::Arc};

use figures::{Displayable, Figure, Round};

use crate::{
    color::Color,
    math::{Pixels, Point, Scaled},
    scene::{Element, Target},
    text::Font,
};

/// A vertical metrics measurement.
#[derive(Copy, Clone, Debug)]
pub struct VMetrics {
    /// The amount of pixels above the baseline.
    pub ascent: Figure<f32, Pixels>,
    /// The amount of pixels below the baseline. Typically a negative number.
    pub descent: Figure<f32, Pixels>,
    /// The amount of pixels to allow between lines.
    pub line_gap: Figure<f32, Pixels>,
}

impl From<rusttype::VMetrics> for VMetrics {
    fn from(value: rusttype::VMetrics) -> Self {
        Self {
            ascent: Figure::new(value.ascent),
            descent: Figure::new(value.descent),
            line_gap: Figure::new(value.line_gap),
        }
    }
}

impl VMetrics {
    /// The total height of the line.
    #[must_use]
    pub fn line_height(&self) -> Figure<f32, Pixels> {
        self.height() + self.line_gap
    }

    /// The height of the ascent and descent combined.
    #[must_use]
    pub fn height(&self) -> Figure<f32, Pixels> {
        self.ascent - self.descent
    }
}

/// A formatted span of text that is ready to render. Cheap to clone.
#[derive(Clone, Debug)]
pub struct PreparedSpan {
    /// The location of the span.
    pub location: Point<f32, Pixels>,
    data: Arc<PreparedSpanData>,
}

impl PreparedSpan {
    #[must_use]
    pub(crate) fn new(
        font: Font,
        size: Figure<f32, Pixels>,
        color: Color,
        width: Figure<f32, Pixels>,
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
    pub(crate) fn translate(&self, location: Point<f32, Pixels>) -> Self {
        Self {
            // We want to ensure that we are pixel-aligned when rendering a span's start.
            location: location.round(),
            data: self.data.clone(),
        }
    }

    /// Renders the text in `scene` with the baseline at `location`
    pub fn render_baseline_at(
        &self,
        scene: &Target,
        location: Point<f32, Scaled>,
    ) -> crate::Result<()> {
        let effective_scale_factor = scene.scale();

        let location =
            scene.offset_point_raw(location.to_pixels(effective_scale_factor) + self.location);
        scene.push_element(Element::Text {
            span: self.translate(location),
            clip: scene.clip,
        });
        Ok(())
    }
}

impl Deref for PreparedSpan {
    type Target = PreparedSpanData;

    fn deref(&self) -> &Self::Target {
        self.data.as_ref()
    }
}

/// The shared data of a [`PreparedSpan`].
#[derive(Debug)]
pub struct PreparedSpanData {
    /// The font being rendered.
    pub font: Font,
    /// The font size.
    pub size: Figure<f32, Pixels>,
    /// The color to render.
    pub color: Color,
    /// The total width of the span.
    pub width: Figure<f32, Pixels>,
    /// THe characters that compose this span.
    pub characters: Vec<char>,
    /// The glyphs that will be rendered.
    pub glyphs: Vec<GlyphInfo>,
    /// The vertical metrics of the span.
    pub metrics: rusttype::VMetrics,
}

/// Information about a font glyph
#[derive(Debug)]
pub struct GlyphInfo {
    /// The offset of the glyph within the source string.
    pub source_offset: usize,
    /// The character responsible for this glyph.
    pub source: char,
    /// The positioned glyph.
    pub glyph: rusttype::PositionedGlyph<'static>,
}

impl GlyphInfo {
    /// The width of the glyph.
    #[must_use]
    pub fn width(&self) -> Figure<f32, Pixels> {
        Figure::new(self.glyph.unpositioned().h_metrics().advance_width)
    }

    /// The location of the glyph, relative to the span start.
    #[must_use]
    pub fn location(&self) -> Point<f32, Pixels> {
        Point::new(self.glyph.position().x, self.glyph.position().y)
    }
}
