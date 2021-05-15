use std::ops::Range;

use stylecs::Style;

use crate::{
    math::{Point, Raw, Scaled},
    scene::Target,
    KludgineResult,
};

#[cfg(feature = "bundled-fonts-enabled")]
pub mod bundled_fonts;
pub mod font;
pub mod prepared;
pub mod rich;
pub mod wrap;
use font::*;
use prepared::*;
use wrap::*;

#[derive(Debug, Clone, Default)]
pub struct Span {
    pub text: String,
    pub style: Style<Raw>,
}

impl Span {
    pub fn new<S: Into<String>>(text: S, style: Style<Raw>) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    spans: Vec<Span>,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            spans: vec![Span::default()],
        }
    }
}

impl Text {
    pub fn span<S: Into<String>>(text: S, style: Style<Raw>) -> Self {
        Self::new(vec![Span::new(text, style)])
    }

    pub fn new(spans: Vec<Span>) -> Self {
        Self { spans }
    }

    pub fn wrap(&self, scene: &Target, options: TextWrap) -> KludgineResult<PreparedText> {
        TextWrapper::wrap(self, scene, options)
    }

    pub fn render_at(
        &self,
        scene: &Target,
        location: Point<f32, Scaled>,
        wrapping: TextWrap,
    ) -> KludgineResult<()> {
        self.render_core(scene, location, true, wrapping)
    }

    pub fn render_baseline_at(
        &self,
        scene: &Target,
        location: Point<f32, Scaled>,
        wrapping: TextWrap,
    ) -> KludgineResult<()> {
        self.render_core(scene, location, false, wrapping)
    }

    fn render_core(
        &self,
        scene: &Target,
        location: Point<f32, Scaled>,
        offset_baseline: bool,
        wrapping: TextWrap,
    ) -> KludgineResult<()> {
        let prepared_text = self.wrap(scene, wrapping)?;
        prepared_text
            .render(scene, location, offset_baseline)
            .map(|_| ())
    }

    pub fn remove_range(&mut self, range: Range<usize>) {
        self.for_each_in_range_mut(range, |span, relative_range| {
            span.text.replace_range(relative_range, "");
        })
    }

    pub fn insert_str(&mut self, offset: usize, value: &str) {
        self.for_each_in_range_mut(offset..offset + 1, |span, relative_range| {
            span.text.insert_str(relative_range.start, value);
        })
    }

    pub fn len(&self) -> usize {
        self.spans.iter().map(|s| s.text.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn for_each_in_range<F: FnMut(&Span, Range<usize>)>(
        &self,
        range: Range<usize>,
        mut callback: F,
    ) {
        let mut span_start = 0usize;
        for span in self.spans.iter() {
            let span_len = span.text.len();
            let span_end = span_start + span_len;

            if span_end >= range.start {
                if span_start >= range.end {
                    return;
                }

                let relative_range =
                    (range.start - span_start).max(0)..(range.end - span_start).min(span_len);
                callback(span, relative_range);
            }

            span_start = span_end;
        }
    }

    pub fn for_each_in_range_mut<F: Fn(&mut Span, Range<usize>)>(
        &mut self,
        range: Range<usize>,
        callback: F,
    ) {
        let mut span_start = 0usize;
        for span in self.spans.iter_mut() {
            let span_len = span.text.len();
            let span_end = span_start + span_len;

            if span_end >= range.start {
                if span_start >= range.end {
                    break;
                }

                let relative_range = range.start.checked_sub(span_start).unwrap_or_default()
                    ..(range.end.checked_sub(span_start).unwrap_or_default()).min(span_len);
                callback(span, relative_range);
            }

            span_start = span_end;
        }

        self.cleanup_spans();
    }

    fn cleanup_spans(&mut self) {
        if self.is_empty() {
            // If we have no actual text in this, keep the first span and dump the rest
            // Doing this operation separately allows the other branch to be a simple retain
            // operation
            self.spans.resize_with(1, || unreachable!())
        } else {
            self.spans.retain(|span| !span.text.is_empty());
        }
    }
}

impl ToString for Text {
    fn to_string(&self) -> String {
        self.spans
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_remove_one_span_partial() {
        let mut text = Text::span("123456789", Default::default());
        text.remove_range(0..1);
        assert_eq!(text.spans.len(), 1);
        assert_eq!(text.spans[0].text, "23456789");
    }

    #[test]
    fn test_remove_one_span_entire() {
        let mut text = Text::span("1", Default::default());
        text.remove_range(0..1);
        assert_eq!(text.spans.len(), 1);
        assert!(text.spans[0].text.is_empty());
    }

    #[test]
    fn test_remove_multi_span_entire_first() {
        let mut text = Text::new(vec![
            Span::new("1", Default::default()),
            Span::new("2", Default::default()),
            Span::new("3", Default::default()),
        ]);
        text.remove_range(0..1);
        assert_eq!(text.spans.len(), 2);
        assert_eq!(text.spans[0].text, "2");
        assert_eq!(text.spans[1].text, "3");
    }

    #[test]
    fn test_remove_multi_span_entire_middle() {
        let mut text = Text::new(vec![
            Span::new("1", Default::default()),
            Span::new("2", Default::default()),
            Span::new("3", Default::default()),
        ]);
        text.remove_range(1..2);
        assert_eq!(text.spans.len(), 2);
        assert_eq!(text.spans[0].text, "1");
        assert_eq!(text.spans[1].text, "3");
    }

    #[test]
    fn test_remove_multi_span_entire_last() {
        let mut text = Text::new(vec![
            Span::new("1", Default::default()),
            Span::new("2", Default::default()),
            Span::new("3", Default::default()),
        ]);
        text.remove_range(2..3);
        assert_eq!(text.spans.len(), 2);
        assert_eq!(text.spans[0].text, "1");
        assert_eq!(text.spans[1].text, "2");
    }

    #[test]
    fn test_remove_multi_span_multi() {
        let mut text = Text::new(vec![
            Span::new("123a", Default::default()),
            Span::new("b", Default::default()),
            Span::new("c456", Default::default()),
        ]);
        text.remove_range(3..6);
        assert_eq!(text.spans.len(), 2);
        assert_eq!(text.spans[0].text, "123");
        assert_eq!(text.spans[1].text, "456");
    }

    #[test]
    fn test_insert_start() {
        let mut text = Text::span("2", Default::default());
        text.insert_str(0, "1");
        assert_eq!(text.spans.len(), 1);
        assert_eq!(text.spans[0].text, "12");
    }

    #[test]
    fn test_insert_end() {
        let mut text = Text::span("1", Default::default());
        text.insert_str(1, "2");
        assert_eq!(text.spans.len(), 1);
        assert_eq!(text.spans[0].text, "12");
    }
}
