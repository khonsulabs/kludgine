use crate::{
    color::Color,
    math::{Pixels, Point, Points, Raw, Scaled, ScreenScale, Size, SizeExt, Vector},
    scene::Element,
    scene::Scene,
    style::Alignment,
    text::Font,
    KludgineResult,
};
use futures::future::join_all;
use std::sync::Arc;

#[derive(Default, Debug, Clone)]
pub struct PreparedText {
    pub lines: Vec<PreparedLine>,
}

impl PreparedText {
    pub async fn size(&self) -> Size<f32, Raw> {
        let line_sizes = join_all(self.lines.iter().map(|line| line.size())).await;
        let (width, height) = line_sizes.into_iter().fold(
            (Pixels::default(), Pixels::default()),
            |(width, height), line_size| {
                (width.max(line_size.width()), height + line_size.height())
            },
        );
        Size::from_lengths(width, height)
    }

    pub(crate) async fn align(
        &mut self,
        alignment: Alignment,
        width: Points,
        effective_scale: ScreenScale,
    ) {
        let line_sizes = join_all(self.lines.iter().map(|line| line.size())).await;

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

    pub async fn render(
        &self,
        scene: &Scene,
        location: Point<f32, Scaled>,
        offset_baseline: bool,
    ) -> KludgineResult<()> {
        let mut current_line_baseline = Points::new(0.);
        let effective_scale_factor = scene.scale_factor().await;

        if offset_baseline && !self.lines.is_empty() {
            current_line_baseline += self.lines[0].metrics.ascent / effective_scale_factor;
        }

        for line in self.lines.iter() {
            let metrics = line.metrics;
            let cursor_position =
                location + Vector::from_lengths(line.alignment_offset, current_line_baseline);
            for span in line.spans.iter() {
                let location = (cursor_position
                    + span.location.to_vector() / effective_scale_factor)
                    * effective_scale_factor;
                scene
                    .push_element(Element::Text(span.translate(location)))
                    .await;
            }
            current_line_baseline +=
                (metrics.ascent - metrics.descent + metrics.line_gap) / effective_scale_factor;
        }

        Ok(())
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

#[derive(Debug, Clone)]
pub struct PreparedLine {
    pub spans: Vec<PreparedSpan>,
    pub metrics: VMetrics,
    pub alignment_offset: Points,
}

impl PreparedLine {
    pub async fn size(&self) -> Size<f32, Raw> {
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
        self.metrics.ascent - self.metrics.descent + self.metrics.line_gap
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
        positioned_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
        metrics: rusttype::VMetrics,
    ) -> Self {
        Self {
            location: Point::default(),
            data: Arc::new(PreparedSpanData {
                font,
                size,
                color,
                width,
                positioned_glyphs,
                metrics,
            }),
        }
    }

    pub fn translate(&self, location: Point<f32, Raw>) -> Self {
        Self {
            location,
            data: self.data.clone(),
        }
    }

    pub(crate) async fn metrics(&self) -> rusttype::VMetrics {
        self.data.font.metrics(self.data.size).await
    }
}

#[derive(Debug)]
pub struct PreparedSpanData {
    pub font: Font,
    pub size: Pixels,
    pub color: Color,
    pub width: Pixels,
    pub positioned_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
    pub metrics: rusttype::VMetrics,
}
