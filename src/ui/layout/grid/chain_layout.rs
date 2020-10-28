use crate::{
    math::{Dimension, Points, Rect, Scaled, Size, Surround},
    ui::{Layout, LayoutContext, LayoutSolver},
    KludgineResult,
};
use async_trait::async_trait;
use generational_arena::Index;
use std::{collections::HashMap, fmt::Debug, ops::Deref};

#[derive(Debug, Default)]
pub struct ChainLayout {
    elements: Vec<ChainElement>,
}

impl ChainLayout {
    pub fn element<I: Into<ChainElementContents>>(mut self, child: I, height: Dimension) -> Self {
        self.elements.push(ChainElement::new(child.into(), height));
        self
    }

    pub fn for_each_laid_out_element<F: FnMut(&ChainElementContents, Points, Points)>(
        &self,
        full_size: Points,
        mut callback: F,
    ) {
        let mut automatic_measurements = 0usize;
        let mut defined_size = Points::default();

        for element in self.elements.iter() {
            match element.size {
                Dimension::Auto => automatic_measurements += 1,
                Dimension::Length(points) => defined_size += points,
            }
        }

        let remaining_size = (full_size - defined_size).max(Points::default());
        let automatic_size = remaining_size / automatic_measurements as f32;

        let mut x = Points::default();
        for column in self.elements.iter() {
            let remaining_size = full_size - x;
            let size = column
                .size
                .length()
                .unwrap_or(automatic_size)
                .min(remaining_size);

            callback(&column.contents, x, size);
            x += size;
        }
    }
}

#[derive(Debug)]
struct ChainElement {
    pub contents: ChainElementContents,
    pub size: Dimension,
}

#[derive(Debug)]
pub enum ChainElementContents {
    Index(Index),
    Chain(Box<dyn ChainElementDynamicContents>),
}

impl From<Index> for ChainElementContents {
    fn from(index: Index) -> Self {
        Self::Index(index)
    }
}

impl<T> From<T> for ChainElementContents
where
    T: ChainElementDynamicContents + Sized + 'static,
{
    fn from(dynamic: T) -> Self {
        Self::Chain(Box::new(dynamic))
    }
}

pub trait ChainElementDynamicContents: Send + Sync + Debug {
    fn layouts_within_bounds(&self, bounds: &Rect<f32, Scaled>) -> HashMap<Index, Layout>;
}

impl ChainElement {
    pub fn new<C: Into<ChainElementContents>>(contents: C, width: Dimension) -> Self {
        Self {
            contents: contents.into(),
            size: width,
        }
    }
}

pub trait ChainElementDimensionTranslator {
    fn convert_to_margin(min: Points, max: Points) -> Surround<f32, Scaled>;
    fn length_from_bounds(bounds: &Rect<f32, Scaled>) -> Points;
}

impl<T> ChainElementDynamicContents for T
where
    T: Deref<Target = ChainLayout> + ChainElementDimensionTranslator + Debug + Send + Sync,
{
    fn layouts_within_bounds(&self, bounds: &Rect<f32, Scaled>) -> HashMap<Index, Layout> {
        let mut layouts = HashMap::new();
        let full_size = Self::length_from_bounds(bounds);
        self.for_each_laid_out_element(full_size, |contents, position, size| {
            let margin = T::convert_to_margin(position, full_size - size - position);

            match contents {
                ChainElementContents::Index(index) => {
                    layouts.insert(
                        *index,
                        Layout {
                            bounds: *bounds,
                            margin,
                            padding: Default::default(),
                        },
                    );
                }
                ChainElementContents::Chain(dynamic_contents) => {
                    let bounds = margin.inset_rect(bounds);
                    for (index, layout) in dynamic_contents.layouts_within_bounds(&bounds) {
                        layouts.insert(index, layout);
                    }
                }
            }
        });

        layouts
    }
}

#[async_trait]
impl<T> LayoutSolver for T
where
    T: Deref<Target = ChainLayout>
        + ChainElementDynamicContents
        + ChainElementDimensionTranslator
        + Debug
        + Send
        + Sync,
{
    async fn layout_within(
        &self,
        bounds: &Rect<f32, Scaled>,
        _content_size: &Size<f32, Scaled>,
        context: &LayoutContext,
    ) -> KludgineResult<()> {
        let layouts = self
            .layouts_within_bounds(bounds)
            .into_iter()
            .map(|(child, layout)| context.insert_layout(child, layout))
            .collect::<Vec<_>>();

        futures::future::join_all(layouts).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_auto_column() {
        let mut layouts = Vec::new();
        ChainLayout::default()
            .element(Index::from_raw_parts(0, 0), Dimension::Auto)
            .for_each_laid_out_element(Points::new(100.), |_, pos, length| {
                layouts.push((pos.get() as u32, length.get() as u32));
            });

        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0], (0, 100));
    }

    #[test]
    fn two_auto_columns() {
        let mut layouts = Vec::new();
        ChainLayout::default()
            .element(Index::from_raw_parts(0, 0), Dimension::Auto)
            .element(Index::from_raw_parts(0, 1), Dimension::Auto)
            .for_each_laid_out_element(Points::new(100.), |_, pos, length| {
                layouts.push((pos.get() as u32, length.get() as u32));
            });

        assert_eq!(layouts.len(), 2);
        assert_eq!(layouts[0], (0, 50));
        assert_eq!(layouts[1], (50, 50));
    }

    #[test]
    fn two_auto_columns_one_fixed_smaller() {
        let mut layouts = Vec::new();
        ChainLayout::default()
            .element(Index::from_raw_parts(0, 0), Dimension::Auto)
            .element(Index::from_raw_parts(0, 1), Dimension::from_f32(30.))
            .element(Index::from_raw_parts(0, 2), Dimension::Auto)
            .for_each_laid_out_element(Points::new(100.), |_, pos, length| {
                layouts.push((pos.get() as u32, length.get() as u32));
            });

        assert_eq!(layouts.len(), 3);
        assert_eq!(layouts[0], (0, 35));
        assert_eq!(layouts[1], (35, 30));
        assert_eq!(layouts[2], (65, 35));
    }

    #[test]
    fn two_auto_columns_one_fixed_larger() {
        let mut layouts = Vec::new();
        ChainLayout::default()
            .element(Index::from_raw_parts(0, 0), Dimension::Auto)
            .element(Index::from_raw_parts(0, 1), Dimension::from_f32(70.))
            .element(Index::from_raw_parts(0, 2), Dimension::Auto)
            .for_each_laid_out_element(Points::new(100.), |_, pos, length| {
                layouts.push((pos.get() as u32, length.get() as u32));
            });

        assert_eq!(layouts.len(), 3);
        assert_eq!(layouts[0], (0, 15));
        assert_eq!(layouts[1], (15, 70));
        assert_eq!(layouts[2], (85, 15));
    }

    #[test]
    fn too_big() {
        let mut layouts = Vec::new();
        ChainLayout::default()
            .element(Index::from_raw_parts(0, 0), Dimension::Auto)
            .element(Index::from_raw_parts(0, 1), Dimension::from_f32(45.))
            .element(Index::from_raw_parts(0, 2), Dimension::from_f32(45.))
            .element(Index::from_raw_parts(0, 3), Dimension::Auto)
            .for_each_laid_out_element(Points::new(50.), |_, pos, length| {
                layouts.push((pos.get() as u32, length.get() as u32));
            });

        assert_eq!(layouts.len(), 4);
        assert_eq!(layouts[0], (0, 0));
        assert_eq!(layouts[1], (0, 45));
        assert_eq!(layouts[2], (45, 5));
        assert_eq!(layouts[3], (50, 0));
    }
}
