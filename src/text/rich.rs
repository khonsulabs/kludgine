use std::{cmp::Ordering, ops::Range};

use crate::{text::Text, ui::StyledContext, KludgineResult};
use async_handle::Handle;

use super::{prepared::PreparedText, wrap::TextWrap};

#[derive(Debug, Clone)]
pub struct RichText {
    data: Handle<RichTextData>,
}

#[derive(Debug)]
struct RichTextData {
    paragraphs: Vec<Text>,
}

pub enum ParagraphRemoval {
    Remove,
    Keep,
}

impl RichText {
    pub fn new(paragraphs: Vec<Text>) -> Self {
        Self {
            data: Handle::new(RichTextData { paragraphs }),
        }
    }

    pub async fn remove_range(&self, range: Range<RichTextPosition>) {
        assert!(range.start <= range.end);

        self.for_each_in_range_mut(range.clone(), |text, text_range, paragraph_index| {
            text.remove_range(text_range);
            if paragraph_index != range.start.paragraph && paragraph_index != range.end.paragraph {
                ParagraphRemoval::Remove
            } else {
                ParagraphRemoval::Keep
            }
        })
        .await;
    }

    pub async fn insert_str(&self, location: RichTextPosition, value: &str) {
        self.for_each_in_range_mut(location..location, |text, text_range, _| {
            text.insert_str(text_range.start, value);

            ParagraphRemoval::Keep
        })
        .await;
    }

    pub async fn for_each_in_range<F: Fn(&Text, Range<usize>)>(
        &self,
        range: Range<RichTextPosition>,
        callback: F,
    ) {
        let data = self.data.read().await;
        for paragraph_index in range.start.paragraph..(range.end.paragraph + 1) {
            if let Some(paragraph) = data.paragraphs.get(paragraph_index) {
                let relative_range = if range.start.paragraph == paragraph_index {
                    range.start.offset..paragraph.len()
                } else if range.end.paragraph == paragraph_index {
                    0..paragraph.len().min(range.end.offset + 1)
                } else {
                    0..paragraph.len()
                };

                callback(paragraph, relative_range)
            }
        }
    }

    pub async fn for_each_in_range_mut<
        F: Fn(&mut Text, Range<usize>, usize) -> ParagraphRemoval,
    >(
        &self,
        range: Range<RichTextPosition>,
        callback: F,
    ) {
        let mut data = self.data.write().await;
        let mut paragraphs_to_remove = Vec::new();
        for paragraph_index in range.start.paragraph..(range.end.paragraph + 1) {
            if let Some(paragraph) = data.paragraphs.get_mut(paragraph_index) {
                let relative_range = if range.start.paragraph == paragraph_index {
                    range.start.offset..paragraph.len()
                } else if range.end.paragraph == paragraph_index {
                    0..paragraph.len().min(range.end.offset + 1)
                } else {
                    0..paragraph.len()
                };

                if matches!(
                    callback(paragraph, relative_range, paragraph_index),
                    ParagraphRemoval::Remove
                ) {
                    paragraphs_to_remove.push(paragraph_index);
                }
            }
        }

        // Remove in reverse order to ensure that indexes don't change while removing
        paragraphs_to_remove.reverse();
        for paragraph_index in paragraphs_to_remove {
            data.paragraphs.remove(paragraph_index);
        }
    }

    pub async fn prepare(
        &self,
        context: &mut StyledContext,
        wrapping: TextWrap,
    ) -> KludgineResult<Vec<PreparedText>> {
        let data = self.data.read().await;
        let mut prepared = Vec::new();
        for paragraph in data.paragraphs.iter() {
            prepared.push(paragraph.wrap(context.scene(), wrapping.clone()).await?);
        }
        Ok(prepared)
    }

    pub async fn position_after(&self, mut position: RichTextPosition) -> RichTextPosition {
        let data = self.data.read().await;
        let next_offset = position.offset + 1;
        if next_offset > data.paragraphs[position.paragraph].len() {
            if data.paragraphs.len() > position.paragraph + 1 {
                todo!("Need to support multiple paragraphs")
            }
        } else {
            position.offset = next_offset;
        }
        position
    }

    pub async fn position_before(&self, mut position: RichTextPosition) -> RichTextPosition {
        if position.offset == 0 {
            if position.paragraph > 0 {
                todo!("Need to support multiple paragraphs")
            }
        } else {
            position.offset -= 1;
        }

        position
    }

    pub async fn end(&self) -> RichTextPosition {
        let data = self.data.read().await;
        RichTextPosition {
            paragraph: data.paragraphs.len() - 1,
            offset: data.paragraphs.last().unwrap().len(),
        }
    }

    pub async fn to_string(&self) -> String {
        let data = self.data.read().await;
        let mut paragraphs = Vec::with_capacity(data.paragraphs.len());
        for paragraph in data.paragraphs.iter() {
            paragraphs.push(paragraph.to_string());
        }

        paragraphs.join("\n")
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
