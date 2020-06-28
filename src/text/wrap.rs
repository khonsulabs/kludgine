use crate::{
    math::{max_f, min_f},
    scene::SceneTarget,
    style::EffectiveStyle,
    text::{font::Font, PreparedLine, PreparedSpan, PreparedText, Span, Text},
    KludgineResult,
};
use rusttype::{PositionedGlyph, Scale};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum LexerState {
    /// We have wrapped to a new line
    AtLineStart,
    /// We have received at least one glyph for this word
    InWord,
    /// We have encountered a punctuation mark after a word.
    TrailingPunctuation,
    /// We have encountered a whitespace or punctuation character
    AfterWord,
}

pub struct TextWrapper<'a, 'b> {
    caret: f32,
    current_vmetrics: Option<rusttype::VMetrics>,
    last_glyph_id: Option<rusttype::GlyphId>,
    options: TextWrap,
    scene: &'a mut SceneTarget<'b>,
    prepared_text: PreparedText,
    lexer_state: LexerState,
    current_line_spans: Vec<PreparedSpan>,
    current_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
    current_committed_glyphs: Vec<rusttype::PositionedGlyph<'static>>,
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
            current_line_spans: Vec::new(),
            current_glyphs: Vec::new(),
            current_committed_glyphs: Vec::new(),
            current_font: None,
            current_style: None,
            lexer_state: LexerState::AtLineStart,
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
                if let Some(glyph) = self.process_character(c, span, &primary_font).await {
                    if self.lexer_state == LexerState::AtLineStart && c.is_whitespace() {
                        continue;
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
                            || self.current_font.as_ref().unwrap().id != primary_font.id)
                    {
                        self.new_span().await;
                        self.current_font = Some(primary_font.clone());
                        self.current_style = Some(span.style.clone());
                    }

                    self.current_glyphs.push(glyph);
                }
            }

            // Commit the current glyphs to the existing span, since we're getting a new span and styles
            // probably will change.
            self.commit_current_glyphs(None).await;
        }

        self.commit_current_glyphs(None).await;
        self.new_line().await;

        Ok(self.prepared_text)
    }

    async fn process_character(
        &mut self,
        c: char,
        span: &Span,
        primary_font: &Font,
    ) -> Option<PositionedGlyph<'static>> {
        if c.is_control() {
            if c == '\n' {
                // If there's no current line height, we should initialize it with the primary font's height
                if self.current_vmetrics.is_none() {
                    self.current_vmetrics = Some(primary_font.metrics(span.style.font_size).await);
                }

                self.new_line().await;
            }
            return None;
        } else {
            match self.lexer_state {
                LexerState::AtLineStart => {
                    if c.is_whitespace() {
                        return None;
                    } else if c.is_ascii_punctuation() {
                        self.lexer_state = LexerState::AfterWord;
                    } else {
                        self.lexer_state = LexerState::InWord;
                    }
                }
                LexerState::InWord => {
                    if c.is_ascii_punctuation() {
                        self.lexer_state = LexerState::TrailingPunctuation;
                    } else if c.is_whitespace() {
                        self.lexer_state = LexerState::AfterWord;
                    }
                }
                LexerState::TrailingPunctuation => {
                    if c.is_ascii_punctuation() {
                        // This line has been left intentionally blank
                    } else if c.is_whitespace() {
                        self.lexer_state = LexerState::AfterWord;
                    } else {
                        self.commit_current_glyphs(Some(LexerState::InWord)).await;
                    }
                }
                LexerState::AfterWord => {
                    if c.is_ascii_punctuation() {
                        self.lexer_state = LexerState::TrailingPunctuation;
                    } else if !c.is_whitespace() {
                        self.commit_current_glyphs(Some(LexerState::InWord)).await;
                    }
                }
            }
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

        if let Some(max_width) = self.options.max_width(self.scene.effective_scale_factor()) {
            if let Some(bb) = glyph.pixel_bounding_box() {
                if self.current_span_offset + bb.max.x as f32 > max_width {
                    // If the character that is causing us to need to wrap to the next line is whitespace,
                    // the current word should be committed to the current line. If it's punctuation, it belongs to the
                    // word. <-- case in point
                    match self.lexer_state {
                        LexerState::InWord | LexerState::TrailingPunctuation => {
                            // Wrap without committing.
                            // Except, if a single glyph is too wide to draw without being wrapped, return it so that it's
                            // rendered anyways.
                            if self.current_committed_glyphs.len() + self.current_glyphs.len() == 0
                            {
                                return Some(glyph);
                            }
                        }
                        LexerState::AfterWord => {
                            // Commit then wrap.
                            self.commit_current_glyphs(Some(LexerState::AfterWord))
                                .await;
                        }
                        LexerState::AtLineStart => unreachable!(),
                    }

                    self.new_line().await;
                    self.current_font = Some(primary_font.clone());
                    self.current_style = Some(span.style.clone());
                    glyph.set_position(rusttype::point(self.caret, 0.0));
                    self.last_glyph_id = None;
                }
            }
        }
        Some(glyph)
    }

    async fn commit_current_glyphs(&mut self, transition_to_state: Option<LexerState>) {
        if !self.current_glyphs.is_empty() {
            let mut current_glyphs = Vec::new();
            std::mem::swap(&mut self.current_glyphs, &mut current_glyphs);
            self.current_committed_glyphs.extend(current_glyphs);
        }
        if let Some(transition_to_state) = transition_to_state {
            self.lexer_state = transition_to_state;
        }
    }

    async fn new_span(&mut self) {
        if !self.current_committed_glyphs.is_empty() {
            let mut current_style = None;
            std::mem::swap(&mut current_style, &mut self.current_style);
            let current_style = current_style.unwrap();
            let mut current_committed_glyphs = Vec::new();
            std::mem::swap(
                &mut self.current_committed_glyphs,
                &mut current_committed_glyphs,
            );

            let font = self.current_font.as_ref().unwrap().clone();
            let metrics = font.metrics(current_style.font_size).await;
            self.current_line_spans.push(PreparedSpan::new(
                font,
                current_style.font_size,
                current_style.color,
                self.current_span_offset,
                self.caret - self.current_span_offset,
                current_committed_glyphs,
                metrics,
            ));
            self.current_span_offset += self.caret;
            self.caret = 0.0;
        }
    }

    async fn new_line(&mut self) {
        let previous_lexer_state = self.lexer_state;
        self.new_span().await;

        self.lexer_state = LexerState::AtLineStart;
        self.caret = 0.0;
        self.current_span_offset = 0.0;
        self.last_glyph_id = None;

        if !self.current_glyphs.is_empty() {
            // !is_empty()ation for the current glyphs
            let first_offset = self.current_glyphs[0].position().x;
            let mut max_x = 0i32;
            for glyph in self.current_glyphs.iter_mut() {
                let mut positon = glyph.position();
                positon.x -= first_offset;
                glyph.set_position(positon);
                if let Some(bb) = glyph.pixel_bounding_box() {
                    max_x = max_x.max(bb.max.x);
                }
            }
            self.last_glyph_id = Some(self.current_glyphs[self.current_glyphs.len() - 1].id());
            self.caret = max_x as f32;
            self.lexer_state = previous_lexer_state;
        }

        let mut current_line_spans = Vec::new();
        std::mem::swap(&mut current_line_spans, &mut self.current_line_spans);

        let metrics = self.current_vmetrics.unwrap();
        let current_line = PreparedLine {
            spans: current_line_spans,
            metrics,
        };
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

#[cfg(all(test, feature = "bundled-fonts"))]
mod tests {
    use super::*;
    use crate::{scene::Scene, style::Style, text::Span};

    #[async_test]
    /// This test should have "This line should " be on the first line and "wrap" on the second
    async fn wrap_one_word() {
        let mut scene = Scene::default();
        scene.register_bundled_fonts().await;
        let mut scene_target = SceneTarget::Scene(&mut scene);
        let wrap = Text::new(vec![Span::new(
            "This line should wrap",
            Style {
                font_size: Some(12.0),
                ..Default::default()
            }
            .effective_style(&mut scene_target),
        )])
        .wrap(
            &mut scene_target,
            TextWrap::MultiLine {
                width: 80.0,
                height: f32::MAX,
            },
        )
        .await
        .expect("Error wrapping text");
        assert_eq!(wrap.lines.len(), 2);
        assert_eq!(wrap.lines[0].spans.len(), 1);
        assert_eq!(
            wrap.lines[0].spans[0]
                .handle
                .read()
                .await
                .positioned_glyphs
                .len(),
            17
        );
        assert_eq!(wrap.lines[1].spans.len(), 1);
        assert_eq!(
            wrap.lines[1].spans[0]
                .handle
                .read()
                .await
                .positioned_glyphs
                .len(),
            4
        );
    }

    #[async_test]
    /// This test should have "This line should " be on the first line and "wrap" on the second
    async fn wrap_one_word_different_span() {
        let mut scene = Scene::default();
        scene.register_bundled_fonts().await;
        let mut scene_target = SceneTarget::Scene(&mut scene);
        let wrap = Text::new(vec![
            Span::new(
                "This line should ",
                Style {
                    font_size: Some(12.0),
                    ..Default::default()
                }
                .effective_style(&mut scene_target),
            ),
            Span::new(
                "wrap",
                Style {
                    font_size: Some(10.0),
                    ..Default::default()
                }
                .effective_style(&mut scene_target),
            ),
        ])
        .wrap(
            &mut scene_target,
            TextWrap::MultiLine {
                width: 80.0,
                height: f32::MAX,
            },
        )
        .await
        .expect("Error wrapping text");
        assert_eq!(wrap.lines.len(), 2);
        assert_eq!(wrap.lines[0].spans.len(), 1);
        assert_eq!(
            wrap.lines[0].spans[0]
                .handle
                .read()
                .await
                .positioned_glyphs
                .len(),
            17
        );
        assert_eq!(wrap.lines[1].spans.len(), 1);
        assert_eq!(
            wrap.lines[1].spans[0]
                .handle
                .read()
                .await
                .positioned_glyphs
                .len(),
            4
        );
        assert_ne!(
            wrap.lines[0].spans[0].handle.read().await.metrics,
            wrap.lines[1].spans[0].handle.read().await.metrics
        );
    }
}
