use crate::{
    math::{max_f, Point, Size},
    style::Color,
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
}

#[derive(Debug)]
pub struct PreparedLine {
    pub spans: Vec<PreparedSpan>,
    pub metrics: rusttype::VMetrics,
}

impl PreparedLine {
    pub async fn size(&self) -> Size {
        if self.spans.is_empty() {
            return Size::new(0.0, self.height());
        }
        let first = self.spans.get(0).unwrap();
        let last = self.spans.last().unwrap();

        let last_x = last.x().await;
        let last_width = last.width().await;
        let first_x = first.x().await;
        Size::new(last_x + last_width - first_x, self.height())
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

#[derive(Debug)]
pub struct PreparedSpanData {
    pub font: Font,
    pub size: f32,
    pub color: Color,
    pub x: f32,
    pub width: f32,
    pub positioned_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
    pub metrics: rusttype::VMetrics,
}
