use crate::{
    math::Pixels,
    math::Raw,
    math::Scaled,
    scene::Scene,
    style::FallbackStyle,
    style::{FontFamily, FontSize, FontStyle, Style, TextColor, Weight},
    text::{font::Font, PreparedSpan, Text},
    KludgineResult,
};
use euclid::Length;
use rusttype::{GlyphId, PositionedGlyph, Scale};

#[derive(Debug)]
pub(crate) enum Token {
    EndOfLine(rusttype::VMetrics),
    Characters(PreparedSpan),
    Punctuation(PreparedSpan),
    Whitespace(PreparedSpan),
}

#[derive(Debug)]
pub(crate) enum SpanGroup {
    Spans(Vec<PreparedSpan>),
    Whitespace(Vec<PreparedSpan>),
    EndOfLine(rusttype::VMetrics),
}

impl SpanGroup {
    pub(crate) fn spans(&self) -> Vec<PreparedSpan> {
        match self {
            SpanGroup::Spans(spans) => spans.clone(),
            SpanGroup::Whitespace(spans) => spans.clone(),
            SpanGroup::EndOfLine(_) => Vec::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum TokenizerStatus {
    /// We have wrapped to a new line
    AtSpanStart,
    /// We have received at least one glyph for this word
    InWord,
    /// We have encountered a punctuation mark after a word.
    TrailingPunctuation,
    /// We have encountered a whitespace or punctuation character
    Whitespace,
}

#[derive(Default)]
pub(crate) struct Tokenizer {
    tokens: Vec<Token>,
}

struct TokenizerState<'a> {
    style: &'a Style<Raw>,
    font: &'a Font,
    glyphs: Vec<PositionedGlyph<'static>>,
    lexer_state: TokenizerStatus,
    last_glyph_id: Option<GlyphId>,
    caret: Pixels,
}

impl<'a> TokenizerState<'a> {
    pub(crate) fn new(font: &'a Font, style: &'a Style<Raw>) -> Self {
        Self {
            font,
            style,
            lexer_state: TokenizerStatus::AtSpanStart,
            glyphs: Default::default(),
            last_glyph_id: None,
            caret: Pixels::default(),
        }
    }

    async fn emit_token_if_needed(
        &mut self,
        scale: euclid::Scale<f32, Scaled, Raw>,
    ) -> Option<Token> {
        if self.glyphs.is_empty() {
            None
        } else {
            let current_committed_glyphs = std::mem::take(&mut self.glyphs);

            let font_size = style_font_size(&self.style, scale);
            let foreground = TextColor::lookup(self.style).unwrap_or_default().0;
            let metrics = self.font.metrics(font_size).await;
            let span = PreparedSpan::new(
                self.font.clone(),
                font_size,
                foreground,
                self.caret,
                current_committed_glyphs,
                metrics,
            );
            self.caret = Pixels::default();

            let token = match self.lexer_state {
                TokenizerStatus::AtSpanStart => unreachable!(),
                TokenizerStatus::InWord => Token::Characters(span),
                TokenizerStatus::TrailingPunctuation => Token::Punctuation(span),
                TokenizerStatus::Whitespace => Token::Whitespace(span),
            };
            Some(token)
        }
    }
}

impl Tokenizer {
    // Text (Vec<Span>) -> Vec<Token{ PreparedSpan, TokenKind }>
    pub(crate) async fn prepare_spans(
        mut self,
        text: &Text,
        scene: &Scene,
    ) -> KludgineResult<Vec<Token>> {
        let scale = scene.scale_factor().await;
        for span in text.spans.iter() {
            let font = scene
                .lookup_font(
                    &span.style.get_or_default::<FontFamily>().0,
                    span.style.get_or_default::<Weight>(),
                    span.style.get_or_default::<FontStyle>(),
                )
                .await?;
            let vmetrics = font.metrics(style_font_size(&span.style, scale)).await;

            let mut state = TokenizerState::new(&font, &span.style);

            for c in span.text.chars() {
                if c.is_control() {
                    if c == '\n' {
                        self.tokens.push(Token::EndOfLine(vmetrics));
                    }
                } else {
                    let new_lexer_state = if c.is_whitespace() {
                        TokenizerStatus::Whitespace
                    } else if c.is_ascii_punctuation() {
                        TokenizerStatus::TrailingPunctuation
                    } else {
                        TokenizerStatus::InWord
                    };

                    if new_lexer_state != state.lexer_state {
                        if let Some(token) = state.emit_token_if_needed(scale).await {
                            self.tokens.push(token);
                        }
                    }

                    state.lexer_state = new_lexer_state;

                    let base_glyph = font.glyph(c).await;
                    if let Some(id) = state.last_glyph_id.take() {
                        state.caret += Pixels::new(
                            font.pair_kerning(
                                style_font_size(&span.style, scale).get(),
                                id,
                                base_glyph.id(),
                            )
                            .await,
                        );
                    }
                    state.last_glyph_id = Some(base_glyph.id());
                    let glyph = base_glyph
                        .scaled(Scale::uniform(style_font_size(&span.style, scale).get()))
                        .positioned(rusttype::point(state.caret.get(), 0.0));

                    state.caret += Pixels::new(glyph.unpositioned().h_metrics().advance_width);
                    state.glyphs.push(glyph);
                }
            }

            if let Some(token) = state.emit_token_if_needed(scale).await {
                self.tokens.push(token);
            }
        }

        Ok(self.tokens)
    }
}

fn style_font_size(style: &Style<Raw>, scale: euclid::Scale<f32, Scaled, Raw>) -> Length<f32, Raw> {
    style
        .get::<FontSize<Raw>>()
        .cloned()
        .unwrap_or_else(|| FontSize(Length::<f32, Scaled>::new(14.) * scale))
        .0
}
