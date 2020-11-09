use crate::{
    math::{Dimension, Points, Rect, Scaled, Size, SizeExt, Surround},
    ui::{
        layout::{Layout, LayoutSolver},
        Index, Indexable, LayoutContext,
    },
    KludgineError, KludgineResult,
};
use async_trait::async_trait;
use std::collections::HashMap;
#[derive(Default, Debug)]
pub struct AbsoluteLayout {
    children: HashMap<Index, AbsoluteBounds>,
    order: Vec<Index>,
}

impl AbsoluteLayout {
    pub fn child(mut self, index: &impl Indexable, bounds: AbsoluteBounds) -> KludgineResult<Self> {
        let index = index.index();
        self.children.insert(index, bounds.validate()?);
        self.order.push(index);
        Ok(self)
    }

    fn solve_dimension(
        start: &Dimension,
        end: &Dimension,
        length: &Dimension,
        available_length: Points,
        content_length: Points,
    ) -> (Points, Points) {
        let content_length = length.length().unwrap_or(content_length);

        let mut remaining_length = available_length - content_length;

        let mut auto_measurements = 0;
        if let Some(points) = start.length() {
            remaining_length -= points;
        } else {
            auto_measurements += 1;
        }

        if let Some(points) = end.length() {
            remaining_length -= points;
        } else {
            auto_measurements += 1;
        }

        let effective_side1 = match start.length() {
            None => remaining_length.max(Points::default()) / auto_measurements as f32,
            Some(points) => points,
        };

        let effective_side2 = match end.length() {
            None => remaining_length.max(Points::default()) / auto_measurements as f32,
            Some(points) => points,
        };

        remaining_length = available_length - content_length - effective_side1 - effective_side2;

        if remaining_length < Points::default() {
            // The padding was too much, we have an edge case with not enough information
            // Do we decrease the padding or do we decrease the width?
            // For now, the choice is to decrease the width
            let content_length = available_length - effective_side1 - effective_side2;
            if content_length < Points::default() {
                // Ok, we really really are in a pickle. At this point, it almost doesn't matter what we do, because the rendered
                // content size is already 0, so we'll just return 0 for the width and divide the sides evenly *shrug*
                (available_length / 2., available_length / 2.)
            } else {
                (effective_side1, effective_side2)
            }
        } else {
            // If the dimension is auto, increase the width of the content.
            // If the dimension isn't auto, increase the padding
            match length {
                Dimension::Minimal | Dimension::Auto => (effective_side1, effective_side2),
                Dimension::Length(_) => (
                    effective_side1 + remaining_length / 2.,
                    effective_side2 + remaining_length / 2.,
                ),
            }
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct AbsoluteBounds {
    pub left: Dimension,
    pub right: Dimension,
    pub top: Dimension,
    pub bottom: Dimension,
    pub width: Dimension,
    pub height: Dimension,
}

impl AbsoluteBounds {
    pub fn with_left<D: Into<Dimension>>(mut self, dimension: D) -> Self {
        self.left = dimension.into();
        self
    }

    pub fn with_right<D: Into<Dimension>>(mut self, dimension: D) -> Self {
        self.right = dimension.into();
        self
    }

    pub fn with_top<D: Into<Dimension>>(mut self, dimension: D) -> Self {
        self.top = dimension.into();
        self
    }

    pub fn with_bottom<D: Into<Dimension>>(mut self, dimension: D) -> Self {
        self.bottom = dimension.into();
        self
    }

    pub fn with_height<D: Into<Dimension>>(mut self, dimension: D) -> Self {
        self.height = dimension.into();
        self
    }

    pub fn with_width<D: Into<Dimension>>(mut self, dimension: D) -> Self {
        self.width = dimension.into();
        self
    }

    fn validate(self) -> KludgineResult<Self> {
        if self.left.is_length() && self.right.is_length() && self.width.is_length() {
            Err(KludgineError::AbsoluteBoundsInvalidHorizontal)
        } else if self.top.is_length() && self.bottom.is_length() && self.height.is_length() {
            Err(KludgineError::AbsoluteBoundsInvalidVertical)
        } else {
            Ok(self)
        }
    }
}

impl From<Surround<Dimension>> for AbsoluteBounds {
    fn from(surround: Surround<Dimension>) -> Self {
        Self {
            left: surround.left.get(),
            right: surround.right.get(),
            top: surround.top.get(),
            bottom: surround.bottom.get(),
            ..Default::default()
        }
    }
}

impl From<Surround<f32, Scaled>> for AbsoluteBounds {
    fn from(surround: Surround<f32, Scaled>) -> Self {
        Self {
            left: Dimension::from_length(surround.left),
            right: Dimension::from_length(surround.right),
            top: Dimension::from_length(surround.top),
            bottom: Dimension::from_length(surround.bottom),
            ..Default::default()
        }
    }
}

#[async_trait]
impl LayoutSolver for AbsoluteLayout {
    async fn layout_within(
        &self,
        bounds: &Rect<f32, Scaled>,
        _content_size: &Size<f32, Scaled>,
        padding: &Surround<f32, Scaled>,
        context: &LayoutContext,
    ) -> KludgineResult<()> {
        let bounds = padding.inset_rect(bounds);
        for (index, child_bounds) in self
            .order
            .iter()
            .map(|&index| (index, self.children.get(&index).unwrap()))
        {
            let mut child_context = context.clone_for(&index).await;
            let content_size = Size::from_lengths(
                bounds.size.width()
                    - child_bounds.left.length().unwrap_or_default()
                    - child_bounds.right.length().unwrap_or_default(),
                bounds.size.height()
                    - child_bounds.top.length().unwrap_or_default()
                    - child_bounds.bottom.length().unwrap_or_default(),
            );
            let (child_content_size, child_padding) = context
                .arena()
                .get(&index)
                .await
                .unwrap()
                .content_size_with_padding(
                    child_context.styled_context(),
                    &Size::new(Some(content_size.width), Some(content_size.height)),
                )
                .await?;

            let (left, right) = Self::solve_dimension(
                &child_bounds.left,
                &child_bounds.right,
                &child_bounds.width,
                bounds.size.width() - child_padding.minimum_width(),
                child_content_size.width(),
            );
            let (top, bottom) = Self::solve_dimension(
                &child_bounds.top,
                &child_bounds.bottom,
                &child_bounds.height,
                bounds.size.height() - child_padding.minimum_height(),
                child_content_size.height(),
            );

            context
                .insert_layout(
                    index,
                    Layout {
                        bounds,
                        padding: child_padding,
                        margin: Surround {
                            left,
                            top,
                            right,
                            bottom,
                        },
                    },
                )
                .await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    macro_rules! assert_dimension_eq {
        ($left:expr, $check:expr) => {
            assert_relative_eq!($left.0.get(), $check.0);
            assert_relative_eq!($left.1.get(), $check.1)
        };
    }

    #[test]
    fn solve_dimension_tests() -> KludgineResult<()> {
        // start.auto end.auto length.auto
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Auto,
                &Dimension::Auto,
                &Dimension::Auto,
                Points::new(90.),
                Points::new(30.),
            ),
            (30., 30.)
        );

        // start.pts  end.auto length.auto
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::from_f32(50.),
                &Dimension::Auto,
                &Dimension::Auto,
                Points::new(90.),
                Points::new(30.),
            ),
            (50., 10.)
        );

        // start.pts end.pts length.auto
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::from_f32(50.),
                &Dimension::from_f32(0.),
                &Dimension::Auto,
                Points::new(90.),
                Points::new(30.),
            ),
            (50., 0.)
        );

        // start.pts end.auto length.pts
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::from_f32(10.),
                &Dimension::Auto,
                &Dimension::from_f32(10.),
                Points::new(90.),
                Points::new(30.),
            ),
            (10., 70.)
        );

        // start.pts end.pts length.pts
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::from_f32(10.),
                &Dimension::from_f32(75.),
                &Dimension::from_f32(5.),
                Points::new(90.),
                Points::new(30.),
            ),
            (10., 75.)
        );

        // start.auto end.pts length.auto
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Auto,
                &Dimension::from_f32(50.),
                &Dimension::Auto,
                Points::new(90.),
                Points::new(30.),
            ),
            (10., 50.)
        );
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Auto,
                &Dimension::from_f32(50.),
                &Dimension::Auto,
                Points::new(90.),
                Points::new(90.),
            ),
            (0., 50.)
        );

        // start.auto end.pts length.pts
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Auto,
                &Dimension::from_f32(50.),
                &Dimension::from_f32(20.),
                Points::new(90.),
                Points::new(30.),
            ),
            (20., 50.)
        );

        // Running out of room: Not enough space to honor both width and padding
        // This engine's decision is to honor making the "whitespace" layout look laid out correctly
        // for as long as possible, and start clipping the content.
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::from_f32(40.),
                &Dimension::from_f32(40.),
                &Dimension::from_f32(30.),
                Points::new(90.),
                Points::new(30.),
            ),
            (40., 40.)
        );

        // Running out of room: Not even enough room for padding
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::from_f32(50.),
                &Dimension::from_f32(50.),
                &Dimension::from_f32(30.),
                Points::new(90.),
                Points::new(30.),
            ),
            (45., 45.)
        );

        Ok(())
    }

    #[test]
    fn validate_tests() -> KludgineResult<()> {
        AbsoluteBounds {
            bottom: Dimension::from_f32(1.),
            height: Dimension::from_f32(1.),
            top: Dimension::from_f32(1.),
            ..Default::default()
        }
        .validate()
        .expect_err("Invalid Vertical Bounds");
        AbsoluteBounds {
            left: Dimension::from_f32(1.),
            width: Dimension::from_f32(1.),
            right: Dimension::from_f32(1.),
            ..Default::default()
        }
        .validate()
        .expect_err("Invalid Horizontal Bounds");

        Ok(())
    }

    #[async_test]
    async fn layout_test() -> KludgineResult<()> {
        use crate::{
            scene::{Scene, Target},
            style::{theme::Minimal, StyleSheet},
            ui::{
                Component, HierarchicalArena, Layout, LayoutEngine, LayoutSolver, LayoutSolverExt,
                Node, StandaloneComponent, StyledContext, UIState,
            },
        };
        use async_trait::async_trait;
        use std::collections::HashSet;

        #[derive(Debug)]
        struct TestRoot {
            child: Index,
        }
        #[async_trait]
        impl Component for TestRoot {
            async fn layout(
                &mut self,
                _context: &mut StyledContext,
            ) -> KludgineResult<Box<dyn LayoutSolver>> {
                AbsoluteLayout::default()
                    .child(
                        &self.child,
                        AbsoluteBounds {
                            right: Dimension::from_f32(30.),
                            bottom: Dimension::from_f32(30.),
                            width: Dimension::from_f32(90.),
                            height: Dimension::from_f32(90.),
                            ..Default::default()
                        },
                    )?
                    .layout()
            }
        }
        impl StandaloneComponent for TestRoot {}

        #[derive(Debug)]
        struct TestChild {
            other_child: Option<Index>,
        }
        #[async_trait]
        impl Component for TestChild {
            async fn layout(
                &mut self,
                _context: &mut StyledContext,
            ) -> KludgineResult<Box<dyn LayoutSolver>> {
                if let Some(child) = self.other_child {
                    AbsoluteLayout::default()
                        .child(
                            &child,
                            AbsoluteBounds {
                                left: Dimension::from_f32(10.),
                                top: Dimension::from_f32(10.),
                                right: Dimension::from_f32(10.),
                                bottom: Dimension::from_f32(10.),
                                ..Default::default()
                            },
                        )?
                        .layout()
                } else {
                    Layout::none().layout()
                }
            }
        }
        impl StandaloneComponent for TestChild {}

        let arena = HierarchicalArena::default();
        let node = Node::new(
            TestChild { other_child: None },
            StyleSheet::default(),
            AbsoluteBounds::default(),
            true,
            None,
        );
        let leaf = arena.insert(None, node).await;
        let node = Node::new(
            TestChild {
                other_child: Some(leaf),
            },
            StyleSheet::default(),
            Default::default(),
            true,
            None,
        );
        let child = arena.insert(None, node).await;

        let node = Node::new(
            TestRoot { child },
            StyleSheet::default(),
            Default::default(),
            true,
            None,
        );
        let root = arena.insert(None, node).await;
        arena.set_parent(leaf, Some(child)).await;
        arena.set_parent(child, Some(root)).await;

        let scene = Target::from(Scene::new(Minimal::default().theme()));
        scene.set_internal_size(Size::new(200., 200.)).await;
        let (event_sender, _) = async_channel::unbounded();
        let ui_state = UIState::new(event_sender);
        ui_state.push_layer_from_index(root, &arena, &scene).await?;
        let engine = LayoutEngine::layout(
            &arena,
            &ui_state.top_layer().await,
            &ui_state,
            root,
            &scene,
            HashSet::new(),
        )
        .await?;

        let root_layout = engine.get_layout(&root).await.unwrap();
        let child_layout = engine.get_layout(&child).await.unwrap();
        let leaf_layout = engine.get_layout(&leaf).await.unwrap();

        assert_relative_eq!(root_layout.inner_bounds().origin.x, 0.);
        assert_relative_eq!(root_layout.inner_bounds().origin.y, 0.);
        assert_relative_eq!(root_layout.inner_bounds().size.width, 200.);
        assert_relative_eq!(root_layout.inner_bounds().size.height, 200.);

        assert_relative_eq!(child_layout.inner_bounds().origin.x, 80.);
        assert_relative_eq!(child_layout.inner_bounds().origin.y, 80.);
        assert_relative_eq!(child_layout.inner_bounds().size.width, 90.);
        assert_relative_eq!(child_layout.inner_bounds().size.height, 90.);

        assert_relative_eq!(leaf_layout.inner_bounds().origin.x, 90.);
        assert_relative_eq!(leaf_layout.inner_bounds().origin.y, 90.);
        assert_relative_eq!(leaf_layout.inner_bounds().size.width, 70.);
        assert_relative_eq!(leaf_layout.inner_bounds().size.height, 70.);

        Ok(())
    }
}
