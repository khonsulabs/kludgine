use crate::{
    math::{max_f, min_f},
    scene::SceneTarget,
    style::EffectiveStyle,
    text::{font::Font, PreparedLine, PreparedSpan, PreparedText, Text},
    KludgineResult,
};
use rusttype::Scale;
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
