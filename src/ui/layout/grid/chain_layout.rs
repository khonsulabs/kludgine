use crate::{
    math::{Dimension, Point, Points, Rect, Scaled, Size, Surround},
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
    fn layouts_within_bounds(
        &self,
        bounds: &Rect<f32, Scaled>,
        content_sizes: &HashMap<Index, Size<f32, Scaled>>,
    ) -> (Rect<f32, Scaled>, HashMap<Index, Layout>);
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
    fn length_from_size(size: &Size<f32, Scaled>) -> Points;
    fn size_replacing_length(size: &Size<f32, Scaled>, length: Points) -> Size<f32, Scaled>;
}

impl<T> ChainElementDynamicContents for T
where
    T: Deref<Target = ChainLayout> + ChainElementDimensionTranslator + Debug + Send + Sync,
{
    fn layouts_within_bounds(
        &self,
        bounds: &Rect<f32, Scaled>,
        content_sizes: &HashMap<Index, Size<f32, Scaled>>,
    ) -> (Rect<f32, Scaled>, HashMap<Index, Layout>) {
        let mut established_sizes = Vec::with_capacity(self.elements.len());
        established_sizes.resize_with(self.elements.len(), Default::default);
        let full_size = Self::length_from_size(&bounds.size);

        let mut remaining_size = full_size;
        for (element_index, length, element) in
            self.elements
                .iter()
                .enumerate()
                .filter_map(|(index, element)| {
                    element.size.length().map(|length| (index, length, element))
                })
        {
            let effective_size = length.min(remaining_size);
            match &element.contents {
                ChainElementContents::Index(_) => {
                    established_sizes[element_index] = Some(effective_size);
                }
                ChainElementContents::Chain(dynamic_contents) => {
                    let (inner_bounds, _) = dynamic_contents.layouts_within_bounds(
                        &Rect::new(
                            Point::default(),
                            T::size_replacing_length(&bounds.size, effective_size),
                        ),
                        content_sizes,
                    );
                    established_sizes[element_index] =
                        Some(T::length_from_size(&inner_bounds.size));
                }
            }

            remaining_size -= effective_size;
        }

        // All the hardcoded widths have been established, now we need to handle
        // all the Dimension::Minimal measurements. For these, we want to trust
        // whatever measurement they provide in content_sizes, otherwise we'll
        // treat them as automatic in the final loop.
        for (element_index, element) in self
            .elements
            .iter()
            .enumerate()
            .filter(|(_, element)| matches!(element.size, Dimension::Minimal))
        {
            let effective_size = match &element.contents {
                ChainElementContents::Index(index) => {
                    if let Some(content_size) = content_sizes.get(index) {
                        T::length_from_size(content_size)
                    } else {
                        continue;
                    }
                }
                ChainElementContents::Chain(dynamic_contents) => {
                    let remaining_width_if_auto = remaining_size
                        / established_sizes.iter().filter(|s| s.is_none()).count() as f32;
                    let (inner_bounds, _) = dynamic_contents.layouts_within_bounds(
                        &Rect::new(
                            Point::default(),
                            T::size_replacing_length(&bounds.size, remaining_width_if_auto),
                        ),
                        content_sizes,
                    );
                    T::length_from_size(&inner_bounds.size)
                }
            };

            established_sizes[element_index] = Some(effective_size);

            remaining_size -= effective_size;
        }

        // The final loop will assign hardcoded widths to any that are missing, and insert the layouts
        let mut layouts = HashMap::new();
        let mut full_bounds = Option::<Rect<f32, Scaled>>::None;
        let mut position = Points::default();
        let automatic_width =
            remaining_size / established_sizes.iter().filter(|s| s.is_none()).count() as f32;
        for (element_index, element) in self.elements.iter().enumerate() {
            let size = established_sizes[element_index].unwrap_or(automatic_width);
            let end = full_size - position - size;

            // If the child is a chain, we need to insert all the children layouts
            let margin = T::convert_to_margin(position, end);
            let element_bounds = margin.inset_rect(bounds);
            let element_bounds = match &element.contents {
                ChainElementContents::Index(index) => {
                    layouts.insert(
                        *index,
                        Layout {
                            bounds: *bounds,
                            clip_to: element_bounds,
                            margin,
                            content_offset: None,
                            padding: Default::default(),
                        },
                    );
                    element_bounds
                }
                ChainElementContents::Chain(dynamic_contents) => {
                    let (bounds, child_layouts) =
                        dynamic_contents.layouts_within_bounds(&element_bounds, content_sizes);

                    for (index, layout) in child_layouts {
                        layouts.insert(index, layout);
                    }

                    bounds
                }
            };
            full_bounds = Some(
                full_bounds
                    .map(|b| b.union(&element_bounds))
                    .unwrap_or(element_bounds),
            );

            position += size;
        }

        (full_bounds.unwrap_or_default(), layouts)
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
        _padding: &Surround<f32, Scaled>,
        context: &LayoutContext,
    ) -> KludgineResult<()> {
        let mut content_sizes = HashMap::new();
        let constraints = Size::new(Some(bounds.size.width), Some(bounds.size.height));
        for child in context.children_of(context.index()).await {
            let mut child_context = context.clone_for(&child).await;
            let child_content_size = context
                .arena()
                .get(&child)
                .await
                .unwrap()
                .content_size(child_context.styled_context(), &constraints)
                .await?;
            content_sizes.insert(child, child_content_size);
        }

        let layouts = self
            .layouts_within_bounds(bounds, &content_sizes)
            .1
            .into_iter()
            .map(|(child, layout)| context.insert_layout(child, layout))
            .collect::<Vec<_>>();

        futures::future::join_all(layouts).await;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LayoutMeasurement {
    pub dimension: Dimension,
    pub size: Points,
}
