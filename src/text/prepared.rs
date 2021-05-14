use crate::{
    color::Color,
    math::{Pixels, Point, Points, Raw, Scaled, ScreenScale, Size, SizeExt, Vector},
    scene::{Element, Target},
    text::Font,
    KludgineResult,
};
use std::sync::Arc;
use stylecs::Alignment;

#[derive(Default, Debug, Clone)]
pub struct PreparedText {
    pub lines: Vec<PreparedLine>,
}

impl PreparedText {
    pub fn size(&self) -> Size<f32, Raw> {
        let line_sizes = self.lines.iter().map(|line| line.size());
        let (width, height) = line_sizes.fold(
            (Pixels::default(), Pixels::default()),
            |(width, height), line_size| {
                (width.max(line_size.width()), height + line_size.height())
            },
        );
        Size::from_lengths(width, height)
    }

    #[allow(clippy::needless_collect)] // The collect gets id of the borrow.
    pub(crate) fn align(
        &mut self,
        alignment: Alignment,
        width: Points,
        effective_scale: ScreenScale,
    ) {
        let line_sizes = self
            .lines
            .iter()
            .map(|line| line.size())
            .collect::<Vec<_>>();

        for (i, size) in line_sizes.into_iter().enumerate() {
            match alignment {
                Alignment::Left => {
                    self.lines[i].alignment_offset = Points::default();
                }
                Alignment::Center => {
                    self.lines[i].alignment_offset =
                        (width - (size.width() / effective_scale)) / 2.;
                }
                Alignment::Right => {
                    self.lines[i].alignment_offset = width - size.width() / effective_scale;
                }
            }
        }
    }

    pub fn render(
        &self,
        scene: &Target<'_>,
        location: Point<f32, Scaled>,
        offset_baseline: bool,
    ) -> KludgineResult<Points> {
        let mut current_line_baseline = Points::new(0.);
        let effective_scale_factor = scene.scale_factor();

        for (line_index, line) in self.lines.iter().enumerate() {
            if offset_baseline || line_index > 0 {
                current_line_baseline += line.metrics.ascent / effective_scale_factor;
            }
            let metrics = line.metrics;
            let cursor_position =
                location + Vector::from_lengths(line.alignment_offset, current_line_baseline);
            for span in line.spans.iter() {
                let location = scene.offset_point_raw(
                    (cursor_position + span.location.to_vector() / effective_scale_factor)
                        * effective_scale_factor,
                );
                scene.push_element(Element::Text {
                    span: span.translate(location),
                    clip: scene.clip,
                });
            }
            current_line_baseline += (metrics.line_gap - metrics.descent) / effective_scale_factor;
        }

        Ok(current_line_baseline)
    }
}

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
    pub fn line_height(&self) -> Pixels {
        self.height() + self.line_gap
    }

    pub fn height(&self) -> Pixels {
        self.ascent - self.descent
    }
}

#[derive(Debug, Clone)]
pub struct PreparedLine {
    pub spans: Vec<PreparedSpan>,
    pub metrics: VMetrics,
    pub alignment_offset: Points,
}

impl PreparedLine {
    pub fn size(&self) -> Size<f32, Raw> {
        if self.spans.is_empty() {
            return Size::from_lengths(Pixels::default(), self.height());
        }

        let width = self
            .spans
            .iter()
            .map(|s| s.data.width)
            .fold(Pixels::default(), |sum, s| sum + s);

        Size::from_lengths(width, self.height())
    }

    pub fn height(&self) -> Pixels {
        self.metrics.line_height()
    }
}

#[derive(Clone, Debug)]
pub struct PreparedSpan {
    pub location: Point<f32, Raw>,
    pub data: Arc<PreparedSpanData>,
}

impl PreparedSpan {
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

    pub fn translate(&self, location: Point<f32, Raw>) -> Self {
        Self {
            // We want to ensure that we are pixel-aligned when rendering a span's start.
            location: location.round(),
            data: self.data.clone(),
        }
    }

    pub(crate) fn metrics(&self) -> rusttype::VMetrics {
        self.data.font.metrics(self.data.size)
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
    pub fn width(&self) -> Pixels {
        Pixels::new(self.glyph.unpositioned().h_metrics().advance_width)
    }

    pub fn location(&self) -> Point<f32, Raw> {
        Point::new(self.glyph.position().x, self.glyph.position().y)
    }
}
