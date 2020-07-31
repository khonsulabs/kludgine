use crate::{
    math::{Dimension, Rect, Size, Surround},
    ui::{
        layout::{Layout, LayoutSolver},
        Index, LayoutContext,
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
    pub fn child(
        mut self,
        index: impl Into<Index>,
        bounds: AbsoluteBounds,
    ) -> KludgineResult<Self> {
        let index = index.into();
        self.children.insert(index, bounds.validate()?);
        self.order.push(index);
        Ok(self)
    }

    fn solve_dimension(
        start: &Dimension,
        end: &Dimension,
        length: &Dimension,
        available_length: f32,
        content_length: f32,
    ) -> (f32, f32) {
        let content_length = length.points().unwrap_or(content_length);

        let mut remaining_length = available_length - content_length;

        let mut auto_measurements = 0;
        if let Some(points) = start.points() {
            remaining_length -= points;
        } else {
            auto_measurements += 1;
        }

        if let Some(points) = end.points() {
            remaining_length -= points;
        } else {
            auto_measurements += 1;
        }

        let effective_side1 = match start {
            Dimension::Auto => remaining_length.max(0.) / auto_measurements as f32,
            Dimension::Points(points) => *points,
        };

        let effective_side2 = match end {
            Dimension::Auto => remaining_length.max(0.) / auto_measurements as f32,
            Dimension::Points(points) => *points,
        };

        remaining_length = available_length - content_length - effective_side1 - effective_side2;

        if remaining_length < -0. {
            // The padding was too much, we have an edge case with not enough information
            // Do we decrease the padding or do we decrease the width?
            // For now, the choice is to decrease the width
            let content_length = available_length - effective_side1 - effective_side2;
            if content_length < 0. {
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
                Dimension::Auto => (effective_side1, effective_side2),
                Dimension::Points(_) => (
                    effective_side1 + remaining_length / 2.,
                    effective_side2 + remaining_length / 2.,
                ),
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct AbsoluteBounds {
    pub left: Dimension,
    pub right: Dimension,
    pub top: Dimension,
    pub bottom: Dimension,
    pub width: Dimension,
    pub height: Dimension,
}

impl AbsoluteBounds {
    fn validate(self) -> KludgineResult<Self> {
        if self.left.is_points() && self.right.is_points() && self.width.is_points() {
            Err(KludgineError::AbsoluteBoundsInvalidHorizontal)
        } else if self.top.is_points() && self.bottom.is_points() && self.height.is_points() {
            Err(KludgineError::AbsoluteBoundsInvalidVertical)
        } else {
            Ok(self)
        }
    }
}

#[async_trait]
impl LayoutSolver for AbsoluteLayout {
    async fn layout_within(
        &self,
        bounds: &Rect,
        content_size: &Size,
        context: &mut LayoutContext,
    ) -> KludgineResult<()> {
        for (index, child_bounds) in self
            .order
            .iter()
            .map(|&index| (index, self.children.get(&index).unwrap()))
        {
            let mut child_context = context.clone_for(index).await;
            let child_content_size = context
                .arena()
                .get(index)
                .await
                .unwrap()
                .content_size(
                    child_context.styled_context(),
                    &Size::new(Some(content_size.width), Some(content_size.height)),
                )
                .await?;
            let (left, right) = Self::solve_dimension(
                &child_bounds.left,
                &child_bounds.right,
                &child_bounds.width,
                bounds.size.width,
                child_content_size.width,
            );
            let (top, bottom) = Self::solve_dimension(
                &child_bounds.top,
                &child_bounds.bottom,
                &child_bounds.height,
                bounds.size.height,
                child_content_size.height,
            );

            context
                .insert_layout(
                    index,
                    Layout {
                        bounds: *bounds,
                        padding: Default::default(),
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
        ($left:expr, $right:expr) => {
            assert_relative_eq!($left.0, $right.0);
            assert_relative_eq!($left.1, $right.1)
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
                90.,
                30.,
            ),
            (30., 30.)
        );

        // start.pts  end.auto length.auto
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Points(50.),
                &Dimension::Auto,
                &Dimension::Auto,
                90.,
                30.,
            ),
            (50., 10.)
        );

        // start.pts end.pts length.auto
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Points(50.),
                &Dimension::Points(0.),
                &Dimension::Auto,
                90.,
                30.,
            ),
            (50., 0.)
        );

        // start.pts end.auto length.pts
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Points(10.),
                &Dimension::Auto,
                &Dimension::Points(10.),
                90.,
                30.,
            ),
            (10., 70.)
        );

        // start.pts end.pts length.pts
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Points(10.),
                &Dimension::Points(75.),
                &Dimension::Points(5.),
                90.,
                30.,
            ),
            (10., 75.)
        );

        // start.auto end.pts length.auto
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Auto,
                &Dimension::Points(50.),
                &Dimension::Auto,
                90.,
                30.,
            ),
            (10., 50.)
        );
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Auto,
                &Dimension::Points(50.),
                &Dimension::Auto,
                90.,
                90.,
            ),
            (0., 50.)
        );

        // start.auto end.pts length.pts
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Auto,
                &Dimension::Points(50.),
                &Dimension::Points(20.),
                90.,
                30.,
            ),
            (20., 50.)
        );

        // Running out of room: Not enough space to honor both width and padding
        // This engine's decision is to honor making the "whitespace" layout look laid out correctly
        // for as long as possible, and start clipping the content.
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Points(40.),
                &Dimension::Points(40.),
                &Dimension::Points(30.),
                90.,
                30.,
            ),
            (40., 40.)
        );

        // Running out of room: Not even enough room for padding
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Points(50.),
                &Dimension::Points(50.),
                &Dimension::Points(30.),
                90.,
                30.,
            ),
            (45., 45.)
        );

        Ok(())
    }

    #[test]
    fn validate_tests() -> KludgineResult<()> {
        AbsoluteBounds {
            bottom: Dimension::Points(1.),
            height: Dimension::Points(1.),
            top: Dimension::Points(1.),
            ..Default::default()
        }
        .validate()
        .expect_err("Invalid Vertical Bounds");
        AbsoluteBounds {
            left: Dimension::Points(1.),
            width: Dimension::Points(1.),
            right: Dimension::Points(1.),
            ..Default::default()
        }
        .validate()
        .expect_err("Invalid Horizontal Bounds");

        Ok(())
    }

    #[tokio::test]
    async fn layout_test() -> KludgineResult<()> {
        use crate::{
            scene::{Scene, SceneTarget},
            style::StyleSheet,
            ui::{
                Component, HierarchicalArena, Layout, LayoutEngine, LayoutSolver, LayoutSolverExt,
                Node, StandaloneComponent, StyledContext, UIState,
            },
        };
        use async_trait::async_trait;
        use std::collections::HashSet;
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
                        self.child,
                        AbsoluteBounds {
                            right: Dimension::Points(30.),
                            bottom: Dimension::Points(30.),
                            width: Dimension::Points(90.),
                            height: Dimension::Points(90.),
                            ..Default::default()
                        },
                    )?
                    .layout()
            }
        }
        impl StandaloneComponent for TestRoot {}

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
                            child,
                            AbsoluteBounds {
                                left: Dimension::Points(10.),
                                top: Dimension::Points(10.),
                                right: Dimension::Points(10.),
                                bottom: Dimension::Points(10.),
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
        let node = Node::new(TestChild { other_child: None }, StyleSheet::default(), None);
        let leaf = arena.insert(None, node).await;
        let node = Node::new(
            TestChild {
                other_child: Some(leaf),
            },
            StyleSheet::default(),
            None,
        );
        let child = arena.insert(None, node).await;

        let node = Node::new(TestRoot { child }, StyleSheet::default(), None);
        let root = arena.insert(None, node).await;
        arena.set_parent(leaf, Some(child)).await;
        arena.set_parent(child, Some(root)).await;

        let scene = Scene::default();
        scene.set_internal_size(Size::new(200., 200.)).await;
        let scene_target = SceneTarget::Scene(scene);
        let engine = LayoutEngine::layout(
            &arena,
            &UIState::default(),
            root,
            &scene_target,
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
