use super::{
    math::Point,
    scene::{Element, Scene},
    style::{EffectiveStyle, Style},
    KludgineHandle, KludgineResult,
};
use crossbeam::atomic::AtomicCell;
use lazy_static::lazy_static;
use rgx::core::*;
use rusttype::{gpu_cache, Scale};

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
            handle: KludgineHandle::new(FontData {
                font,
                id: GLOBAL_ID_CELL.fetch_add(1),
            }),
        })
    }

    pub fn id(&self) -> u64 {
        let font = self.handle.read().expect("Error reading font");
        font.id
    }

    pub fn metrics(&self, size: f32) -> rusttype::VMetrics {
        let font = self.handle.read().expect("Error reading font");
        font.font.v_metrics(rusttype::Scale::uniform(size))
    }

    pub fn family(&self) -> Option<String> {
        let font = self.handle.read().expect("Error reading font");
        match &font.font {
            rusttype::Font::Ref(f) => f.family_name(),
            _ => None,
        }
    }

    pub fn weight(&self) -> ttf_parser::Weight {
        let font = self.handle.read().expect("Error reading font");
        match &font.font {
            rusttype::Font::Ref(f) => f.weight(),
            _ => ttf_parser::Weight::Normal,
        }
    }

    pub fn glyph(&self, c: char) -> rusttype::Glyph<'static> {
        let font = self.handle.read().expect("Error reading font");
        font.font.glyph(c)
    }

    pub fn pair_kerning(&self, size: f32, a: rusttype::GlyphId, b: rusttype::GlyphId) -> f32 {
        let font = self.handle.read().expect("Error reading font");
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
            handle: KludgineHandle::new(LoadedFontData {
                font: font.clone(),
                cache: gpu_cache::Cache::builder().dimensions(512, 512).build(),
                binding: None,
                texture: None,
            }),
        }
    }
}

pub(crate) struct LoadedFontData {
    pub font: Font,
    pub cache: gpu_cache::Cache<'static>,
    pub(crate) binding: Option<BindingGroup>,
    pub(crate) texture: Option<rgx::core::Texture>,
}

#[derive(Clone)]
pub(crate) struct PreparedSpan {
    pub location: Point,
    pub line_metrics: rusttype::VMetrics,
    pub handle: KludgineHandle<RenderedSpanData>,
}

impl PreparedSpan {
    pub fn new(
        font: Font,
        size: f32,
        color: Rgba,
        location: Point,
        max_width: Option<f32>,
        positioned_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
        line_metrics: rusttype::VMetrics,
    ) -> Self {
        Self {
            location: Point::new(0.0, 0.0),
            line_metrics,
            handle: KludgineHandle::new(RenderedSpanData {
                font,
                size,
                color,
                location,
                max_width,
                positioned_glyphs,
            }),
        }
    }

    pub fn translate(&self, location: Point, line_metrics: rusttype::VMetrics) -> Self {
        Self {
            location,
            line_metrics,
            handle: self.handle.clone(),
        }
    }
}

pub struct RenderedSpanData {
    pub font: Font,
    pub size: f32,
    pub color: Rgba,
    pub location: Point,
    pub max_width: Option<f32>,
    pub positioned_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
}

#[derive(Debug)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

impl Span {
    pub fn new<S: Into<String>>(text: S, style: Style) -> Self {
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
    pub fn span<S: Into<String>>(text: S, style: Style) -> Self {
        Self {
            spans: vec![Span::new(text, style)],
        }
    }

    pub fn new(spans: Vec<Span>) -> Self {
        Self { spans }
    }

    pub fn wrap(&self, scene: &mut Scene, options: TextWrap) -> KludgineResult<PreparedText> {
        TextWrapper::wrap(self, scene, options) // TODO cache result
    }

    pub fn render_at(
        &self,
        scene: &mut Scene,
        location: Point,
        wrapping: TextWrap,
    ) -> KludgineResult<()> {
        let prepared_text = self.wrap(scene, wrapping)?;
        let mut current_line_top = 0.0;

        for line in prepared_text.lines.iter() {
            let metrics = line.metrics.as_ref().unwrap();
            let cursor_position = Point::new(location.x, location.y + current_line_top);
            for span in line.spans.iter() {
                scene.elements.push(Element::Text(span.translate(
                    scene.user_to_device_point(cursor_position) * scene.effective_scale_factor(),
                    *metrics,
                )));
            }
            current_line_top =
                current_line_top + metrics.ascent - metrics.descent + metrics.line_gap;
        }

        Ok(())
    }
}

pub struct TextWrapper<'a> {
    caret: rusttype::Point<f32>,
    current_vmetrics: Option<rusttype::VMetrics>,
    last_glyph_id: Option<rusttype::GlyphId>,
    options: TextWrap,
    scene: &'a mut Scene,
    prepared_text: PreparedText,
    current_line: PreparedLine,
    current_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
    current_font: Option<Font>,
    current_style: Option<EffectiveStyle>,
    current_span_location: rusttype::Point<f32>,
}

impl<'a> TextWrapper<'a> {
    pub fn wrap(text: &Text, scene: &mut Scene, options: TextWrap) -> KludgineResult<PreparedText> {
        TextWrapper {
            caret: rusttype::point(0.0, 0.0),
            current_span_location: rusttype::point(0.0, 0.0),
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
    }

    fn wrap_text(mut self, text: &Text) -> KludgineResult<PreparedText> {
        for span in text.spans.iter() {
            let effective_style = span.style.effective_style();
            if self.current_style.is_none() {
                self.current_style = Some(effective_style.clone());
            } else if self.current_style.as_ref() != Some(&effective_style) {
                self.new_span();
                self.current_style = Some(effective_style.clone());
            }

            let primary_font = self
                .scene
                .lookup_font(&effective_style.font_family, effective_style.font_weight)?;

            for c in span.text.chars() {
                if c.is_control() {
                    match c {
                        '\n' => {
                            // If there's no current line height, we should initialize it with the primary font's height
                            if self.current_vmetrics.is_none() {
                                self.current_vmetrics =
                                    Some(primary_font.metrics(effective_style.font_size));
                            }

                            self.new_line();
                        }
                        _ => {}
                    }
                    continue;
                }

                let base_glyph = primary_font.glyph(c);
                if let Some(id) = self.last_glyph_id.take() {
                    self.caret.x +=
                        primary_font.pair_kerning(effective_style.font_size, id, base_glyph.id());
                }
                self.last_glyph_id = Some(base_glyph.id());
                let mut glyph = base_glyph
                    .scaled(Scale::uniform(effective_style.font_size))
                    .positioned(self.caret);

                if let Some(max_width) = self.options.max_width(self.scene.effective_scale_factor())
                {
                    if let Some(bb) = glyph.pixel_bounding_box() {
                        if self.current_span_location.x + bb.max.x as f32 > max_width {
                            self.new_line();
                            glyph.set_position(self.caret);
                            self.last_glyph_id = None;
                        }
                    }
                }

                if self.current_vmetrics.is_none() {
                    self.current_vmetrics = Some(primary_font.metrics(effective_style.font_size));
                }

                self.caret.x += glyph.unpositioned().h_metrics().advance_width;

                if (self.current_style.is_none()
                    || self.current_style.as_ref() != Some(&effective_style))
                    || (self.current_font.is_none()
                        || self.current_font.as_ref().unwrap().id() != primary_font.id())
                {
                    self.new_span();
                    self.current_font = Some(primary_font.clone());
                    self.current_style = Some(effective_style.clone());
                }

                self.current_glyphs.push(glyph);
            }
        }

        self.new_line();

        Ok(self.prepared_text)
    }

    fn new_span(&mut self) {
        if self.current_glyphs.len() > 0 {
            let mut current_style = None;
            std::mem::swap(&mut current_style, &mut self.current_style);
            let current_style = current_style.unwrap();
            let mut current_glyphs = Vec::new();
            std::mem::swap(&mut self.current_glyphs, &mut current_glyphs);
            self.current_line.spans.push(PreparedSpan::new(
                self.current_font.as_ref().unwrap().clone(),
                current_style.font_size,
                current_style.color,
                Point::new(self.current_span_location.x, self.current_span_location.y),
                self.options.max_width(self.scene.effective_scale_factor()),
                current_glyphs,
                self.current_vmetrics.unwrap(),
            ));
            self.current_span_location = rusttype::point(
                self.caret.x + self.current_span_location.x,
                self.current_span_location.y,
            );
            self.caret = rusttype::point(0.0, self.caret.y);
        }
    }

    fn new_line(&mut self) {
        self.new_span();

        let metrics = self.current_vmetrics.unwrap();
        self.caret = rusttype::point(
            0.0,
            self.caret.y + metrics.ascent - metrics.descent + metrics.line_gap,
        );
        self.current_span_location = self.caret;

        self.current_vmetrics = None;
        let mut current_line = PreparedLine::default();
        std::mem::swap(&mut current_line, &mut self.current_line);
        current_line.metrics = Some(metrics);
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

#[derive(Default)]
pub struct PreparedLine {
    spans: Vec<PreparedSpan>,
    metrics: Option<rusttype::VMetrics>,
}
