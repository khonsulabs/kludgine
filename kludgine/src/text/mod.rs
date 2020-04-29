use super::{
    math::{max_f, min_f, Point, Size},
    scene::{Element, SceneTarget},
    style::EffectiveStyle,
    KludgineHandle, KludgineResult,
};
use async_std::sync::RwLock;
use crossbeam::atomic::AtomicCell;
use futures::future::join_all;
use lazy_static::lazy_static;
use rgx::core::*;
use rusttype::{gpu_cache, Scale};
use std::sync::Arc;

#[cfg(feature = "bundled-fonts-enabled")]
pub mod bundled_fonts;

lazy_static! {
    static ref GLOBAL_ID_CELL: AtomicCell<u64> = { AtomicCell::new(0) };
}

/// Font provides TrueType Font rendering
#[derive(Clone)]
pub struct Font {
    pub(crate) handle: KludgineHandle<FontData>,
}

impl Font {
    pub fn try_from_bytes(bytes: &'static [u8]) -> Option<Font> {
        let font = rusttype::Font::try_from_bytes(bytes)?;
        Some(Font {
            handle: Arc::new(RwLock::new(FontData {
                font,
                id: GLOBAL_ID_CELL.fetch_add(1),
            })),
        })
    }

    pub async fn id(&self) -> u64 {
        let font = self.handle.read().await;
        font.id
    }

    pub async fn metrics(&self, size: f32) -> rusttype::VMetrics {
        let font = self.handle.read().await;
        font.font.v_metrics(rusttype::Scale::uniform(size))
    }

    pub async fn family(&self) -> Option<String> {
        let font = self.handle.read().await;
        match &font.font {
            rusttype::Font::Ref(f) => f.family_name(),
            _ => None,
        }
    }

    pub async fn weight(&self) -> ttf_parser::Weight {
        let font = self.handle.read().await;
        match &font.font {
            rusttype::Font::Ref(f) => f.weight(),
            _ => ttf_parser::Weight::Normal,
        }
    }

    pub async fn glyph(&self, c: char) -> rusttype::Glyph<'static> {
        let font = self.handle.read().await;
        font.font.glyph(c)
    }

    pub async fn pair_kerning(&self, size: f32, a: rusttype::GlyphId, b: rusttype::GlyphId) -> f32 {
        let font = self.handle.read().await;
        font.font.pair_kerning(Scale::uniform(size), a, b)
    }
}

pub(crate) struct FontData {
    pub(crate) id: u64,
    pub(crate) font: rusttype::Font<'static>,
}

#[derive(Clone)]
pub(crate) struct LoadedFont {
    pub handle: KludgineHandle<LoadedFontData>,
}

impl LoadedFont {
    pub fn new(font: &Font) -> Self {
        Self {
            handle: Arc::new(RwLock::new(LoadedFontData {
                font: font.clone(),
                cache: gpu_cache::Cache::builder().dimensions(512, 512).build(),
                binding: None,
                texture: None,
            })),
        }
    }
}

pub(crate) struct LoadedFontData {
    pub font: Font,
    pub cache: gpu_cache::Cache<'static>,
    pub(crate) binding: Option<BindingGroup>,
    pub(crate) texture: Option<rgx::core::Texture>,
}

#[derive(Debug)]
pub struct Span {
    pub text: String,
    pub style: EffectiveStyle,
}

impl Span {
    pub fn new<S: Into<String>>(text: S, style: EffectiveStyle) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }
}

#[derive(Debug)]
pub struct Text {
    spans: Vec<Span>,
}

impl Text {
    pub fn span<S: Into<String>>(text: S, style: &EffectiveStyle) -> Self {
        Self {
            spans: vec![Span::new(text, style.clone())],
        }
    }

    pub fn new(spans: Vec<Span>) -> Self {
        Self { spans }
    }

    pub async fn wrap<'a>(
        &self,
        scene: &mut SceneTarget<'a>,
        options: TextWrap,
    ) -> KludgineResult<PreparedText> {
        TextWrapper::wrap(self, scene, options).await // TODO cache result
    }

    pub async fn render_at<'a>(
        &self,
        scene: &mut SceneTarget<'a>,
        location: Point,
        wrapping: TextWrap,
    ) -> KludgineResult<()> {
        let prepared_text = self.wrap(scene, wrapping).await?;
        let mut current_line_baseline = 0.0;
        let effective_scale_factor = scene.effective_scale_factor();

        for line in prepared_text.lines.iter() {
            let metrics = line.metrics.as_ref().unwrap();
            let cursor_position = Point::new(location.x, location.y + current_line_baseline);
            for span in line.spans.iter() {
                let mut location = scene
                    .user_to_device_point(Point::new(cursor_position.x, cursor_position.y))
                    * effective_scale_factor;
                location.x += span.x().await;
                scene.push_element(Element::Text(span.translate(location)));
            }
            current_line_baseline = current_line_baseline
                + (metrics.ascent - metrics.descent + metrics.line_gap) / effective_scale_factor;
        }

        Ok(())
    }
}

pub struct TextWrapper<'a, 'b> {
    caret: f32,
    current_vmetrics: Option<rusttype::VMetrics>,
    last_glyph_id: Option<rusttype::GlyphId>,
    options: TextWrap,
    scene: &'a mut SceneTarget<'b>,
    prepared_text: PreparedText,
    current_line: PreparedLine,
    current_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
    current_font: Option<Font>,
    current_style: Option<EffectiveStyle>,
    current_span_offset: f32,
}

impl<'a, 'b> TextWrapper<'a, 'b> {
    pub async fn wrap(
        text: &Text,
        scene: &'a mut SceneTarget<'b>,
        options: TextWrap,
    ) -> KludgineResult<PreparedText> {
        TextWrapper {
            caret: 0.0,
            current_span_offset: 0.0,
            current_vmetrics: None,
            last_glyph_id: None,
            options,
            scene,
            prepared_text: PreparedText::default(),
            current_line: PreparedLine::default(),
            current_glyphs: Vec::new(),
            current_font: None,
            current_style: None,
        }
        .wrap_text(text)
        .await
    }

    async fn wrap_text(mut self, text: &Text) -> KludgineResult<PreparedText> {
        for span in text.spans.iter() {
            if self.current_style.is_none() {
                self.current_style = Some(span.style.clone());
            } else if self.current_style.as_ref() != Some(&span.style) {
                self.new_span().await;
                self.current_style = Some(span.style.clone());
            }

            let primary_font = self
                .scene
                .lookup_font(&span.style.font_family, span.style.font_weight)
                .await?;

            for c in span.text.chars() {
                if c.is_control() {
                    match c {
                        '\n' => {
                            // If there's no current line height, we should initialize it with the primary font's height
                            if self.current_vmetrics.is_none() {
                                self.current_vmetrics =
                                    Some(primary_font.metrics(span.style.font_size).await);
                            }

                            self.new_line().await;
                        }
                        _ => {}
                    }
                    continue;
                }

                let base_glyph = primary_font.glyph(c).await;
                if let Some(id) = self.last_glyph_id.take() {
                    self.caret += primary_font
                        .pair_kerning(span.style.font_size, id, base_glyph.id())
                        .await;
                }
                self.last_glyph_id = Some(base_glyph.id());
                let mut glyph = base_glyph
                    .scaled(Scale::uniform(span.style.font_size))
                    .positioned(rusttype::point(self.caret, 0.0));

                if let Some(max_width) = self.options.max_width(self.scene.effective_scale_factor())
                {
                    if let Some(bb) = glyph.pixel_bounding_box() {
                        if self.current_span_offset + bb.max.x as f32 > max_width {
                            self.new_line().await;
                            glyph.set_position(rusttype::point(self.caret, 0.0));
                            self.last_glyph_id = None;
                        }
                    }
                }

                let metrics = primary_font.metrics(span.style.font_size).await;
                if let Some(current_vmetrics) = &self.current_vmetrics {
                    self.current_vmetrics = Some(rusttype::VMetrics {
                        ascent: max_f(current_vmetrics.ascent, metrics.ascent),
                        descent: min_f(current_vmetrics.descent, metrics.descent),
                        line_gap: max_f(current_vmetrics.line_gap, metrics.line_gap),
                    });
                } else {
                    self.current_vmetrics = Some(metrics);
                }

                self.caret += glyph.unpositioned().h_metrics().advance_width;

                if (self.current_style.is_none()
                    || self.current_style.as_ref() != Some(&span.style))
                    || (self.current_font.is_none()
                        || self.current_font.as_ref().unwrap().id().await
                            != primary_font.id().await)
                {
                    self.new_span().await;
                    self.current_font = Some(primary_font.clone());
                    self.current_style = Some(span.style.clone());
                }

                self.current_glyphs.push(glyph);
            }
        }

        self.new_line().await;

        Ok(self.prepared_text)
    }

    async fn new_span(&mut self) {
        if self.current_glyphs.len() > 0 {
            let mut current_style = None;
            std::mem::swap(&mut current_style, &mut self.current_style);
            let current_style = current_style.unwrap();
            let mut current_glyphs = Vec::new();
            std::mem::swap(&mut self.current_glyphs, &mut current_glyphs);

            let font = self.current_font.as_ref().unwrap().clone();
            let metrics = font.metrics(current_style.font_size).await;
            self.current_line.spans.push(PreparedSpan::new(
                font,
                current_style.font_size,
                current_style.color,
                self.current_span_offset,
                self.caret - self.current_span_offset,
                current_glyphs,
                metrics,
            ));
            self.current_span_offset = self.caret + self.current_span_offset;
            self.caret = 0.0;
        }
    }

    async fn new_line(&mut self) {
        self.new_span().await;

        self.caret = 0.0;
        self.current_span_offset = 0.0;

        let mut current_line = PreparedLine::default();
        std::mem::swap(&mut current_line, &mut self.current_line);

        let metrics = self.current_vmetrics.unwrap();
        current_line.metrics = Some(metrics);
        self.current_vmetrics = None;

        self.prepared_text.lines.push(current_line);
    }
}

#[derive(Debug)]
pub enum TextWrap {
    NoWrap,
    SingleLine { max_width: f32, truncate: bool },
    MultiLine { width: f32, height: f32 },
}

impl TextWrap {
    pub fn is_multiline(&self) -> bool {
        match self {
            Self::MultiLine { .. } => true,
            _ => false,
        }
    }

    pub fn is_single_line(&self) -> bool {
        !self.is_multiline()
    }

    pub fn max_width(&self, scale_factor: f32) -> Option<f32> {
        match self {
            Self::MultiLine { width, .. } => Some(*width * scale_factor),
            Self::SingleLine { max_width, .. } => Some(*max_width * scale_factor),
            Self::NoWrap => None,
        }
    }

    pub fn height(&self, scale_factor: f32) -> Option<f32> {
        match self {
            Self::MultiLine { height, .. } => Some(*height * scale_factor),
            _ => None,
        }
    }

    pub fn truncate(&self) -> bool {
        match self {
            Self::SingleLine { truncate, .. } => *truncate,
            _ => false,
        }
    }
}

#[derive(Default)]
pub struct PreparedText {
    lines: Vec<PreparedLine>,
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
    spans: Vec<PreparedSpan>,
    metrics: Option<rusttype::VMetrics>,
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
pub(crate) struct PreparedSpan {
    pub location: Point,
    pub handle: KludgineHandle<PreparedSpanData>,
}

impl PreparedSpan {
    pub fn new(
        font: Font,
        size: f32,
        color: Rgba,
        x: f32,
        width: f32,
        positioned_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
        metrics: rusttype::VMetrics,
    ) -> Self {
        Self {
            location: Point::new(0.0, 0.0),
            handle: Arc::new(RwLock::new(PreparedSpanData {
                font,
                size,
                color,
                x,
                width,
                positioned_glyphs,
                metrics,
            })),
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
    pub color: Rgba,
    pub x: f32,
    pub width: f32,
    pub positioned_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
    pub metrics: rusttype::VMetrics,
}
