use crate::{
    color::Color,
    math::{max_f, Point, Size},
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
    pub async fn size(&self) -> Size {
        let line_sizes = join_all(self.lines.iter().map(|line| line.size())).await;
        let (width, height) = line_sizes
            .into_iter()
            .fold((0f32, 0f32), |(width, height), line_size| {
                (max_f(width, line_size.width), height + line_size.height)
            });
        Size::new(width, height)
    }

    pub(crate) async fn align(&mut self, alignment: Alignment, width: f32) {
        let line_sizes = join_all(self.lines.iter().map(|line| line.size())).await;

        for (i, size) in line_sizes.into_iter().enumerate() {
            match alignment {
                Alignment::Left => {
                    self.lines[i].alignment_offset = 0.;
                }
                Alignment::Center => {
                    self.lines[i].alignment_offset = (width - size.width) / 2.;
                }
                Alignment::Right => {
                    self.lines[i].alignment_offset = width - size.width;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct PreparedLine {
    pub spans: Vec<PreparedSpan>,
    pub metrics: rusttype::VMetrics,
    pub alignment_offset: f32,
}

impl PreparedLine {
    pub async fn size(&self) -> Size {
        if self.spans.is_empty() {
            return Size::new(0.0, self.height());
        }

        let width = join_all(self.spans.iter().map(|s| s.width()))
            .await
            .into_iter()
            .sum::<f32>();
        Size::new(width, self.height())
    }

    pub fn height(&self) -> f32 {
        self.metrics.ascent - self.metrics.descent + self.metrics.line_gap
    }
}

#[derive(Clone, Debug)]
pub struct PreparedSpan {
    pub location: Point,
    pub handle: KludgineHandle<PreparedSpanData>,
}

impl PreparedSpan {
    pub fn new(
        font: Font,
        size: f32,
        color: Color,
        width: f32,
        positioned_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
        metrics: rusttype::VMetrics,
    ) -> Self {
        Self {
            location: Point::new(0.0, 0.0),
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

    pub fn translate(&self, location: Point) -> Self {
        Self {
            location,
            handle: self.handle.clone(),
        }
    }

    pub async fn width(&self) -> f32 {
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
    pub width: f32,
    pub positioned_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
    pub metrics: rusttype::VMetrics,
}
