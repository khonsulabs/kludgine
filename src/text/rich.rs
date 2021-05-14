use crate::{
    scene::Target,
    text::{prepared::PreparedText, wrap::TextWrap, Text},
    KludgineResult,
};
use std::{
    cmp::Ordering,
    fmt::{Display, Write},
    ops::Range,
};

#[derive(Debug)]
pub struct RichText {
    paragraphs: Vec<Text>,
}

impl Default for RichText {
    fn default() -> Self {
        Self {
            paragraphs: vec![Text::default()],
        }
    }
}

pub enum ParagraphRemoval {
    Remove,
    Keep,
}

impl RichText {
    pub fn new(paragraphs: Vec<Text>) -> Self {
        Self { paragraphs }
    }

    pub fn remove_range(&mut self, range: Range<RichTextPosition>) {
        assert!(range.start <= range.end);

        self.for_each_in_range_mut(range.clone(), |text, text_range, paragraph_index| {
            text.remove_range(text_range);
            if paragraph_index != range.start.paragraph && paragraph_index != range.end.paragraph {
                ParagraphRemoval::Remove
            } else {
                ParagraphRemoval::Keep
            }
        });

        // If the range spanned paragraphs, the inner paragraphs will be removed but we need to
        // merge the first and last paragraphs
        if range.start.paragraph != range.end.paragraph {
            let mut paragraph_to_merge = self.paragraphs.remove(range.start.paragraph + 1);
            self.paragraphs[range.start.paragraph]
                .spans
                .append(&mut paragraph_to_merge.spans);
        }
    }

    pub fn insert_str(&mut self, location: RichTextPosition, value: &str) {
        self.for_each_in_range_mut(location..location, |text, text_range, _| {
            text.insert_str(text_range.start, value);

            ParagraphRemoval::Keep
        });
    }

    pub fn for_each_in_range<F: FnMut(&Text, Range<usize>)>(
        &self,
        range: Range<RichTextPosition>,
        mut callback: F,
    ) {
        for paragraph_index in range.start.paragraph..(range.end.paragraph + 1) {
            if let Some(paragraph) = self.paragraphs.get(paragraph_index) {
                let start = if range.start.paragraph == paragraph_index {
                    range.start.offset
                } else {
                    0
                };
                let end = if range.end.paragraph == paragraph_index {
                    paragraph.len().min(range.end.offset)
                } else {
                    paragraph.len()
                };

                callback(paragraph, start..end)
            }
        }
    }

    pub fn for_each_in_range_mut<F: Fn(&mut Text, Range<usize>, usize) -> ParagraphRemoval>(
        &mut self,
        range: Range<RichTextPosition>,
        callback: F,
    ) {
        let mut paragraphs_to_remove = Vec::new();

        for paragraph_index in range.start.paragraph..(range.end.paragraph + 1) {
            if let Some(paragraph) = self.paragraphs.get_mut(paragraph_index) {
                let start = if range.start.paragraph == paragraph_index {
                    range.start.offset
                } else {
                    0
                };
                let end = if range.end.paragraph == paragraph_index {
                    paragraph.len().min(range.end.offset)
                } else {
                    paragraph.len()
                };

                if matches!(
                    callback(paragraph, start..end, paragraph_index),
                    ParagraphRemoval::Remove
                ) {
                    paragraphs_to_remove.push(paragraph_index);
                }
            }
        }

        // Remove in reverse order to ensure that indexes don't change while removing
        paragraphs_to_remove.reverse();
        for paragraph_index in paragraphs_to_remove {
            self.paragraphs.remove(paragraph_index);
        }
    }

    pub fn prepare(
        &self,
        scene: &Target<'_>,
        wrapping: TextWrap,
    ) -> KludgineResult<Vec<PreparedText>> {
        let mut prepared = Vec::new();
        for paragraph in self.paragraphs.iter() {
            prepared.push(paragraph.wrap(scene, wrapping.clone())?);
        }
        Ok(prepared)
    }

    pub fn position_after(&self, mut position: RichTextPosition) -> RichTextPosition {
        let next_offset = position.offset + 1;
        if next_offset > self.paragraphs[position.paragraph].len() {
            if self.paragraphs.len() > position.paragraph + 1 {
                todo!("Need to support multiple paragraphs")
            }
        } else {
            position.offset = next_offset;
        }
        position
    }

    pub fn position_before(&self, mut position: RichTextPosition) -> RichTextPosition {
        if position.offset == 0 {
            if position.paragraph > 0 {
                todo!("Need to support multiple paragraphs")
            }
        } else {
            position.offset -= 1;
        }

        position
    }

    pub fn end(&self) -> RichTextPosition {
        RichTextPosition {
            paragraph: self.paragraphs.len() - 1,
            offset: self.paragraphs.last().unwrap().len(),
        }
    }

    pub fn paragraphs(&self) -> &'_ Vec<Text> {
        &self.paragraphs
    }
}

impl Display for RichText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, paragraph) in self.paragraphs.iter().enumerate() {
            if index > 0 {
                f.write_char('\n')?;
            }

            f.write_str(&paragraph.to_string())?;
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct RichTextPosition {
    pub paragraph: usize,
    pub offset: usize,
}

impl PartialOrd for RichTextPosition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RichTextPosition {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.paragraph.cmp(&other.paragraph) {
            Ordering::Equal => self.offset.cmp(&other.offset),
            not_equal => not_equal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_range_one_paragraph_start() {
        let mut text = RichText::new(vec![Text::span("a123", Default::default())]);
        text.remove_range(
            RichTextPosition {
                offset: 0,
                paragraph: 0,
            }..RichTextPosition {
                offset: 1,
                paragraph: 0,
            },
        );
        assert_eq!(text.to_string(), "123");
    }

    #[test]
    fn remove_range_one_paragraph_end() {
        let mut text = RichText::new(vec![Text::span("123a", Default::default())]);
        text.remove_range(
            RichTextPosition {
                offset: 3,
                paragraph: 0,
            }..RichTextPosition {
                offset: 4,
                paragraph: 0,
            },
        );
        assert_eq!(text.to_string(), "123");
    }

    #[test]
    fn remove_range_one_paragraph_inner() {
        let mut text = RichText::new(vec![Text::span("1a23", Default::default())]);
        text.remove_range(
            RichTextPosition {
                offset: 1,
                paragraph: 0,
            }..RichTextPosition {
                offset: 2,
                paragraph: 0,
            },
        );
        assert_eq!(text.to_string(), "123");
    }

    #[test]
    fn remove_range_multi_paragraph_cross_boundaries() {
        let mut text = RichText::new(vec![
            Text::span("123a", Default::default()),
            Text::span("b456", Default::default()),
        ]);
        text.remove_range(
            RichTextPosition {
                offset: 3,
                paragraph: 0,
            }..RichTextPosition {
                offset: 1,
                paragraph: 1,
            },
        );
        assert_eq!(text.to_string(), "123456");
    }

    #[test]
    fn remove_range_multi_paragraph_cross_multiple_boundaries() {
        let mut text = RichText::new(vec![
            Text::span("123a", Default::default()),
            Text::span("bc", Default::default()),
            Text::span("d456", Default::default()),
        ]);
        text.remove_range(
            RichTextPosition {
                offset: 3,
                paragraph: 0,
            }..RichTextPosition {
                offset: 1,
                paragraph: 2,
            },
        );
        assert_eq!(text.to_string(), "123456");
    }
}
