use crate::{
    math::{max_f, min_f, Pixels, PointExt, Points},
    scene::SceneTarget,
    style::Alignment,
    text::{PreparedLine, PreparedSpan, PreparedText, Text},
    KludgineResult,
};
use approx::relative_eq;
use futures::future::join_all;

mod measured;
mod tokenizer;
pub(crate) use self::{measured::*, tokenizer::*};

pub struct TextWrapper {
    options: TextWrap,
    scene: SceneTarget,
    prepared_text: PreparedText,
}

pub(crate) enum ParserStatus {
    LineStart,
    InWord,
    TrailingPunctuation,
    Whitespace,
}

struct TextWrapState {
    width: Option<Pixels>,
    current_vmetrics: Option<rusttype::VMetrics>,
    current_span_offset: Pixels,
    current_groups: Vec<SpanGroup>,
    lines: Vec<PreparedLine>,
}

impl TextWrapState {
    async fn push_group(&mut self, group: SpanGroup) {
        if let SpanGroup::EndOfLine(metrics) = &group {
            self.update_vmetrics(*metrics);
            self.new_line().await;
        } else {
            let spans = group.spans();
            let total_width = join_all(spans.iter().map(|s| s.width()))
                .await
                .into_iter()
                .fold(Pixels::default(), |sum, width| sum + width);

            if let Some(width) = self.width {
                let new_width = total_width + self.current_span_offset;
                let remaining_width = width - new_width;

                if !relative_eq!(remaining_width.get(), 0., epsilon = 0.001)
                    && remaining_width.get().is_sign_negative()
                {
                    if relative_eq!(self.current_span_offset.get(), 0.) {
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
        span.location.set_x(self.current_span_offset);
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

        self.current_span_offset = Pixels::default();
        for span in spans.iter_mut() {
            self.update_vmetrics(span.metrics().await);
            self.position_span(span).await
        }

        if let Some(metrics) = self.current_vmetrics.take() {
            let metrics = metrics.into();
            self.lines.push(PreparedLine {
                spans,
                metrics,
                alignment_offset: Points::default(),
            });
        }
        self.current_span_offset = Pixels::default();
        self.current_groups.clear();
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
        let width = self.options.max_width();

        let measured = MeasuredText::new(text, &self.scene).await?;

        let mut state = TextWrapState {
            width: width.map(|w| w * effective_scale_factor),
            current_span_offset: Pixels::default(),
            current_vmetrics: None,
            current_groups: Vec::new(),
            lines: Vec::new(),
        };

        for group in measured.groups {
            state.push_group(group).await;
        }

        self.prepared_text.lines = state.finish().await;

        if let Some(alignment) = self.options.alignment() {
            if let Some(max_width) = self.options.max_width() {
                self.prepared_text
                    .align(alignment, max_width, effective_scale_factor)
                    .await;
            }
        }

        Ok(self.prepared_text)
    }
}

#[derive(Debug)]
pub enum TextWrap {
    NoWrap,
    SingleLine {
        max_width: Points,
        truncate: bool,
        alignment: Alignment,
    },
    MultiLine {
        width: Points,
        height: Points,
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

    pub fn max_width(&self) -> Option<Points> {
        match self {
            Self::MultiLine { width, .. } => Some(*width),
            Self::SingleLine { max_width, .. } => Some(*max_width),
            Self::NoWrap => None,
        }
    }

    pub fn height(&self) -> Option<Points> {
        match self {
            Self::MultiLine { height, .. } => Some(*height),
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
    use crate::{math::ScreenScale, scene::Scene, style::Style, text::Span};

    #[async_test]
    /// This test should have "This line should " be on the first line and "wrap" on the second
    async fn wrap_one_word() {
        for &scale in &[1f32, 2.] {
            let mut scene = Scene::default();
            scene.set_scale_factor(ScreenScale::new(scale)).await;
            scene.register_bundled_fonts().await;
            let scene_target = SceneTarget::Scene(scene);
            let wrap = Text::new(vec![Span::new(
                "This line should wrap",
                Style {
                    font_size: Some(Points::new(12.0)),
                    ..Default::default()
                }
                .effective_style(&scene_target)
                .await,
            )])
            .wrap(
                &scene_target,
                TextWrap::MultiLine {
                    width: Points::new(80.0),
                    height: Points::new(f32::MAX),
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
    }

    #[async_test]
    /// This test should have "This line should " be on the first line and "wrap" on the second
    async fn wrap_one_word_different_span() {
        let mut scene = Scene::default();
        scene.register_bundled_fonts().await;
        let scene_target = SceneTarget::Scene(scene);

        let first_style = Style {
            font_size: Some(Points::new(12.0)),
            ..Default::default()
        }
        .effective_style(&scene_target)
        .await;

        let second_style = Style {
            font_size: Some(Points::new(10.0)),
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
                width: Points::new(80.0),
                height: Points::new(f32::MAX),
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
