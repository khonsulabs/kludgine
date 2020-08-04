use crate::{
    color::Color,
    math::{max_f, Pixels, Point, Points, ScreenMeasurement, Size},
    style::Alignment,
    text::Font,
    KludgineHandle,
};
use futures::future::join_all;

#[derive(Default, Debug)]
pub struct PreparedText {
    pub lines: Vec<PreparedLine>,
}

impl PreparedText {
    pub async fn size(&self) -> Size<Pixels> {
        let line_sizes = join_all(self.lines.iter().map(|line| line.size())).await;
        let (width, height) = line_sizes.into_iter().fold(
            (Pixels::default(), Pixels::default()),
            |(width, height), line_size| (width.max(line_size.width), height + line_size.height),
        );
        Size::new(width, height)
    }

    pub(crate) async fn align(
        &mut self,
        alignment: Alignment,
        width: Points,
        effective_scale: f32,
    ) {
        let line_sizes = join_all(self.lines.iter().map(|line| line.size())).await;

        for (i, size) in line_sizes.into_iter().enumerate() {
            match alignment {
                Alignment::Left => {
                    self.lines[i].alignment_offset = Points::default();
                }
                Alignment::Center => {
                    self.lines[i].alignment_offset =
                        (width - size.width.to_points(effective_scale)) / 2.;
                }
                Alignment::Right => {
                    self.lines[i].alignment_offset = width - size.width.to_points(effective_scale);
                }
            }
        }
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
            ascent: Pixels::from_f32(value.ascent),
            descent: Pixels::from_f32(value.descent),
            line_gap: Pixels::from_f32(value.line_gap),
        }
    }
}

#[derive(Debug)]
pub struct PreparedLine {
    pub spans: Vec<PreparedSpan>,
    pub metrics: VMetrics,
    pub alignment_offset: Points,
}

impl PreparedLine {
    pub async fn size(&self) -> Size<Pixels> {
        if self.spans.is_empty() {
            return Size::new(Pixels::default(), self.height());
        }

        let width = join_all(self.spans.iter().map(|s| s.width()))
            .await
            .into_iter()
            .sum::<Pixels>();
        Size::new(width, self.height())
    }

    pub fn height(&self) -> Pixels {
        self.metrics.ascent - self.metrics.descent + self.metrics.line_gap
    }
}

#[derive(Clone, Debug)]
pub struct PreparedSpan {
    pub location: Point<Pixels>,
    pub handle: KludgineHandle<PreparedSpanData>,
}

impl PreparedSpan {
    pub fn new(
        font: Font,
        size: f32,
        color: Color,
        width: Pixels,
        positioned_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
        metrics: rusttype::VMetrics,
    ) -> Self {
        Self {
            location: Point::default(),
            handle: KludgineHandle::new(PreparedSpanData {
                font,
                size,
                color,
                width,
                positioned_glyphs,
                metrics,
            }),
        }
    }

    pub fn translate(&self, location: Point<Pixels>) -> Self {
        Self {
            location,
            handle: self.handle.clone(),
        }
    }

    pub async fn width(&self) -> Pixels {
        let handle = self.handle.read().await;
        handle.width
    }

    pub(crate) async fn metrics(&self) -> rusttype::VMetrics {
        let handle = self.handle.read().await;
        handle.font.metrics(handle.size).await
    }
}

#[derive(Debug)]
pub struct PreparedSpanData {
    pub font: Font,
    pub size: f32,
    pub color: Color,
    pub width: Pixels,
    pub positioned_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
    pub metrics: rusttype::VMetrics,
}
