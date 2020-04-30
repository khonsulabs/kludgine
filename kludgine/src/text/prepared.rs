use crate::{
    math::{max_f, Point, Size},
    style::Color,
    text::Font,
    KludgineHandle,
};
use futures::future::join_all;

#[derive(Default)]
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
}

#[derive(Default)]
pub struct PreparedLine {
    pub spans: Vec<PreparedSpan>,
    pub metrics: Option<rusttype::VMetrics>,
}

impl PreparedLine {
    pub async fn size(&self) -> Size {
        if self.spans.len() == 0 {
            return Size::new(0.0, self.height());
        }
        let first = self.spans.get(0).unwrap();
        let last = self.spans.get(self.spans.len() - 1).unwrap();
        Size::new(
            last.x().await + last.width().await - first.x().await,
            self.height(),
        )
    }

    pub fn height(&self) -> f32 {
        let metrics = self.metrics.as_ref().unwrap();
        metrics.ascent - metrics.descent + metrics.line_gap
    }
}

#[derive(Clone)]
pub struct PreparedSpan {
    pub location: Point,
    pub handle: KludgineHandle<PreparedSpanData>,
}

impl PreparedSpan {
    pub fn new(
        font: Font,
        size: f32,
        color: Color,
        x: f32,
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
                x,
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

    pub async fn x(&self) -> f32 {
        let handle = self.handle.read().await;
        handle.x
    }

    pub async fn width(&self) -> f32 {
        let handle = self.handle.read().await;
        handle.width
    }
}

pub struct PreparedSpanData {
    pub font: Font,
    pub size: f32,
    pub color: Color,
    pub x: f32,
    pub width: f32,
    pub positioned_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
    pub metrics: rusttype::VMetrics,
}
