use crate::{
    math::{max_f, min_f},
    scene::SceneTarget,
    style::{Alignment, EffectiveStyle},
    text::{font::Font, PreparedLine, PreparedSpan, PreparedText, Text},
    KludgineResult,
};
use approx::relative_eq;
use futures::future::join_all;
use rusttype::{GlyphId, PositionedGlyph, Scale};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum LexerStatus {
    /// We have wrapped to a new line
    AtSpanStart,
    /// We have received at least one glyph for this word
    InWord,
    /// We have encountered a punctuation mark after a word.
    TrailingPunctuation,
    /// We have encountered a whitespace or punctuation character
    Whitespace,
}

#[derive(Debug)]
enum Token {
    EndOfLine(rusttype::VMetrics),
    Characters(PreparedSpan),
    Punctuation(PreparedSpan),
    Whitespace(PreparedSpan),
}

#[derive(Debug)]
enum SpanGroup {
    Spans(Vec<PreparedSpan>),
    Whitespace(Vec<PreparedSpan>),
    EndOfLine(rusttype::VMetrics),
}

impl SpanGroup {
    fn spans(&self) -> Vec<PreparedSpan> {
        match self {
            SpanGroup::Spans(spans) => spans.clone(),
            SpanGroup::Whitespace(spans) => spans.clone(),
            SpanGroup::EndOfLine(_) => Vec::default(),
        }
    }
}

#[derive(Default)]
pub struct Lexer {
    tokens: Vec<Token>,
}

struct LexerState<'a> {
    style: &'a EffectiveStyle,
    font: &'a Font,
    glyphs: Vec<PositionedGlyph<'static>>,
    lexer_state: LexerStatus,
    last_glyph_id: Option<GlyphId>,
    caret: f32,
}

impl<'a> LexerState<'a> {
    fn new(font: &'a Font, style: &'a EffectiveStyle) -> Self {
        Self {
            font,
            style,
            lexer_state: LexerStatus::AtSpanStart,
            glyphs: Default::default(),
            last_glyph_id: None,
            caret: 0.,
        }
    }

    async fn emit_token_if_needed(&mut self) -> Option<Token> {
        if self.glyphs.is_empty() {
            None
        } else {
            let current_committed_glyphs = std::mem::take(&mut self.glyphs);

            let metrics = self.font.metrics(self.style.font_size).await;
            let span = PreparedSpan::new(
                self.font.clone(),
                self.style.font_size,
                self.style.color,
                self.caret,
                current_committed_glyphs,
                metrics,
            );
            self.caret = 0.0;

            let token = match self.lexer_state {
                LexerStatus::AtSpanStart => unreachable!(),
                LexerStatus::InWord => Token::Characters(span),
                LexerStatus::TrailingPunctuation => Token::Punctuation(span),
                LexerStatus::Whitespace => Token::Whitespace(span),
            };
            Some(token)
        }
    }
}

impl Lexer {
    // Text (Vec<Span>) -> Vec<Token{ PreparedSpan, TokenKind }>
    async fn prepare_spans(
        mut self,
        text: &Text,
        scene: &SceneTarget,
    ) -> KludgineResult<Vec<Token>> {
        for span in text.spans.iter() {
            let font = scene
                .lookup_font(&span.style.font_family, span.style.font_weight)
                .await?;
            let vmetrics = font.metrics(span.style.font_size).await;

            let mut state = LexerState::new(&font, &span.style);

            for c in span.text.chars() {
                if c.is_control() {
                    if c == '\n' {
                        self.tokens.push(Token::EndOfLine(vmetrics));
                    }
                } else {
                    let new_lexer_state = if c.is_whitespace() {
                        LexerStatus::Whitespace
                    } else if c.is_ascii_punctuation() {
                        LexerStatus::TrailingPunctuation
                    } else {
                        LexerStatus::InWord
                    };

                    if new_lexer_state != state.lexer_state {
                        if let Some(token) = state.emit_token_if_needed().await {
                            self.tokens.push(token);
                        }
                    }

                    state.lexer_state = new_lexer_state;

                    let base_glyph = font.glyph(c).await;
                    if let Some(id) = state.last_glyph_id.take() {
                        state.caret += font
                            .pair_kerning(span.style.font_size, id, base_glyph.id())
                            .await;
                    }
                    state.last_glyph_id = Some(base_glyph.id());
                    let glyph = base_glyph
                        .scaled(Scale::uniform(span.style.font_size))
                        .positioned(rusttype::point(state.caret, 0.0));

                    state.caret += glyph.unpositioned().h_metrics().advance_width;
                    state.glyphs.push(glyph);
                }
            }

            if let Some(token) = state.emit_token_if_needed().await {
                self.tokens.push(token);
            }
        }

        Ok(self.tokens)
    }
}

pub struct MeasuredText {
    groups: Vec<SpanGroup>,
}

struct TextMeasureState {
    current_group: Option<SpanGroup>,
    status: ParserStatus,
    groups: Vec<SpanGroup>,
}

impl TextMeasureState {
    fn push_token(&mut self, token: Token) {
        match token {
            Token::EndOfLine(vmetrics) => {
                self.commit_current_group();
                self.groups.push(SpanGroup::EndOfLine(vmetrics));
                self.status = ParserStatus::LineStart;
            }
            Token::Characters(span) => {
                match self.status {
                    ParserStatus::LineStart | ParserStatus::InWord => {
                        self.push_visual_span(span);
                        self.status = ParserStatus::InWord;
                    }

                    ParserStatus::Whitespace | ParserStatus::TrailingPunctuation => {
                        self.commit_current_group();
                        self.push_visual_span(span);
                        self.status = ParserStatus::TrailingPunctuation;
                    }
                };
            }
            Token::Punctuation(span) => match self.status {
                ParserStatus::TrailingPunctuation => {
                    self.push_visual_span(span);
                }
                ParserStatus::LineStart | ParserStatus::InWord => {
                    self.push_visual_span(span);
                    self.status = ParserStatus::TrailingPunctuation;
                }
                ParserStatus::Whitespace => {
                    self.commit_current_group();
                    self.push_visual_span(span);
                    self.status = ParserStatus::TrailingPunctuation;
                }
            },
            Token::Whitespace(span) => match self.status {
                ParserStatus::Whitespace => {
                    self.push_whitespace_span(span);
                }
                _ => {
                    self.commit_current_group();
                    self.push_whitespace_span(span);
                    self.status = ParserStatus::Whitespace;
                }
            },
        }
    }

    fn push_visual_span(&mut self, span: PreparedSpan) {
        if let Some(SpanGroup::Spans(group)) = &mut self.current_group {
            group.push(span);
        } else {
            self.commit_current_group();
            self.current_group = Some(SpanGroup::Spans(vec![span]));
        }
    }

    fn push_whitespace_span(&mut self, span: PreparedSpan) {
        if let Some(SpanGroup::Whitespace(group)) = &mut self.current_group {
            group.push(span);
        } else {
            self.commit_current_group();
            self.current_group = Some(SpanGroup::Whitespace(vec![span]));
        }
    }

    fn commit_current_group(&mut self) {
        if let Some(group) = self.current_group.take() {
            self.groups.push(group);
        }
    }

    fn finish(mut self) -> Vec<SpanGroup> {
        self.commit_current_group();
        self.groups
    }
}

impl MeasuredText {
    pub async fn new(text: &Text, scene: &SceneTarget) -> KludgineResult<Self> {
        let mut measured = Self { groups: Vec::new() };

        measured.measure_text(text, scene).await?;

        //println!("{:#?}", measured.groups);
        Ok(measured)
    }

    async fn measure_text(&mut self, text: &Text, scene: &SceneTarget) -> KludgineResult<()> {
        let mut state = TextMeasureState {
            current_group: None,
            status: ParserStatus::LineStart,
            groups: Vec::new(),
        };

        // Tokens -> "Words" (groups of characters, and where the breaks would happen)
        for token in Lexer::default().prepare_spans(text, scene).await? {
            state.push_token(token);
        }

        self.groups = state.finish();

        Ok(())
    }
}

pub struct TextWrapper {
    options: TextWrap,
    scene: SceneTarget,
    prepared_text: PreparedText,
}

enum ParserStatus {
    LineStart,
    InWord,
    TrailingPunctuation,
    Whitespace,
}

struct TextWrapState {
    width: Option<f32>,
    status: ParserStatus,
    current_vmetrics: Option<rusttype::VMetrics>,
    current_span_offset: f32,
    current_groups: Vec<SpanGroup>,
    lines: Vec<PreparedLine>,
}

impl TextWrapState {
    async fn push_group(&mut self, group: SpanGroup) {
        if let SpanGroup::EndOfLine(metrics) = &group {
            self.update_vmetrics(*metrics);
            self.new_line().await;
        } else {
            let total_width = join_all(group.spans().iter().map(|s| s.width()))
                .await
                .into_iter()
                .sum::<f32>();
            if let Some(width) = self.width {
                if self.current_span_offset + total_width > width {
                    if relative_eq!(self.current_span_offset, 0.) {
                        // TODO Split the group if it can't fit on a single line
                        // For now, just render it anyways.
                    } else {
                        self.new_line().await;
                    }
                }
            }
            self.current_span_offset += total_width;
            self.current_groups.push(group);
        }
    }

    fn update_vmetrics(&mut self, new_metrics: rusttype::VMetrics) {
        self.current_vmetrics = match self.current_vmetrics {
            Some(metrics) => Some(rusttype::VMetrics {
                ascent: max_f(metrics.ascent, new_metrics.ascent),
                descent: min_f(metrics.descent, new_metrics.descent),
                line_gap: max_f(metrics.line_gap, new_metrics.line_gap),
            }),
            None => Some(new_metrics),
        }
    }

    async fn position_span(&mut self, span: &mut PreparedSpan) {
        let width = span.width().await;
        span.location.x = self.current_span_offset;
        self.current_span_offset += width;
    }

    async fn new_line(&mut self) {
        // Remove any whitespace from the end of the line
        while matches!(self.current_groups.last(), Some(SpanGroup::Whitespace(_))) {
            self.current_groups.pop();
        }

        let mut spans = Vec::new();
        for group in self.current_groups.iter() {
            for span in group.spans() {
                spans.push(span);
            }
        }

        self.current_span_offset = 0.;
        for span in spans.iter_mut() {
            self.update_vmetrics(span.metrics().await);
            self.position_span(span).await
        }

        if let Some(metrics) = self.current_vmetrics.take() {
            self.lines.push(PreparedLine {
                spans,
                metrics,
                alignment_offset: 0.,
            });
        }
        self.current_span_offset = 0.;
        self.current_groups.clear();
        self.status = ParserStatus::LineStart;
    }

    async fn finish(mut self) -> Vec<PreparedLine> {
        if !self.current_groups.is_empty() {
            self.new_line().await;
        }

        self.lines
    }
}

impl TextWrapper {
    pub async fn wrap(
        text: &Text,
        scene: &SceneTarget,
        options: TextWrap,
    ) -> KludgineResult<PreparedText> {
        TextWrapper {
            options,
            scene: scene.clone(),
            prepared_text: PreparedText::default(),
        }
        .wrap_text(text)
        .await
    }

    async fn wrap_text(mut self, text: &Text) -> KludgineResult<PreparedText> {
        let effective_scale_factor = self.scene.effective_scale_factor().await;
        let width = self.options.max_width(effective_scale_factor);

        let measured = MeasuredText::new(text, &self.scene).await?;

        let mut state = TextWrapState {
            width,
            current_span_offset: 0.,
            current_vmetrics: None,
            current_groups: Vec::new(),
            lines: Vec::new(),
            status: ParserStatus::LineStart,
        };

        for group in measured.groups {
            state.push_group(group).await;
        }

        self.prepared_text.lines = state.finish().await;

        if let Some(alignment) = self.options.alignment() {
            if let Some(max_width) = self
                .options
                .max_width(self.scene.effective_scale_factor().await)
            {
                self.prepared_text.align(alignment, max_width).await;
            }
        }

        Ok(self.prepared_text)
    }
}

#[derive(Debug)]
pub enum TextWrap {
    NoWrap,
    SingleLine {
        max_width: f32,
        truncate: bool,
        alignment: Alignment,
    },
    MultiLine {
        width: f32,
        height: f32,
        alignment: Alignment,
    },
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

    pub fn alignment(&self) -> Option<Alignment> {
        match self {
            Self::NoWrap => None,
            Self::MultiLine { alignment, .. } => Some(*alignment),
            Self::SingleLine { alignment, .. } => Some(*alignment),
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
        let scene_target = SceneTarget::Scene(scene);
        let wrap = Text::new(vec![Span::new(
            "This line should wrap",
            Style {
                font_size: Some(12.0),
                ..Default::default()
            }
            .effective_style(&scene_target)
            .await,
        )])
        .wrap(
            &scene_target,
            TextWrap::MultiLine {
                width: 80.0,
                height: f32::MAX,
                alignment: Alignment::Left,
            },
        )
        .await
        .expect("Error wrapping text");
        assert_eq!(wrap.lines.len(), 2);
        assert_eq!(wrap.lines[0].spans.len(), 5); // "this"," ","line"," ","should"
        assert_eq!(wrap.lines[1].spans.len(), 1); // "wrap"
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
        let scene_target = SceneTarget::Scene(scene);

        let first_style = Style {
            font_size: Some(12.0),
            ..Default::default()
        }
        .effective_style(&scene_target)
        .await;

        let second_style = Style {
            font_size: Some(10.0),
            ..Default::default()
        }
        .effective_style(&scene_target)
        .await;

        let wrap = Text::new(vec![
            Span::new("This line should ", first_style),
            Span::new("wrap", second_style),
        ])
        .wrap(
            &scene_target,
            TextWrap::MultiLine {
                width: 80.0,
                height: f32::MAX,
                alignment: Alignment::Left,
            },
        )
        .await
        .expect("Error wrapping text");
        assert_eq!(wrap.lines.len(), 2);
        assert_eq!(wrap.lines[0].spans.len(), 5);
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
