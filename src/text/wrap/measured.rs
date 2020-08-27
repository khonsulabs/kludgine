use crate::{
    scene::SceneTarget,
    text::{ParserStatus, PreparedSpan, SpanGroup, Text, Token, Tokenizer},
    KludgineResult,
};

pub struct MeasuredText {
    pub(crate) groups: Vec<SpanGroup>,
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

        Ok(measured)
    }

    async fn measure_text(&mut self, text: &Text, scene: &SceneTarget) -> KludgineResult<()> {
        let mut state = TextMeasureState {
            current_group: None,
            status: ParserStatus::LineStart,
            groups: Vec::new(),
        };

        // Tokens -> "Words" (groups of characters, and where the breaks would happen)
        for token in Tokenizer::default().prepare_spans(text, scene).await? {
            state.push_token(token);
        }

        self.groups = state.finish();

        Ok(())
    }
}
