use crate::{
    math::{Dimension, Points, Rect, Size, Surround},
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
        available_length: Points,
        content_length: Points,
    ) -> (Points, Points) {
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
            Dimension::Auto => remaining_length.max(Points::default()) / auto_measurements as f32,
            Dimension::Points(points) => *points,
        };

        let effective_side2 = match end {
            Dimension::Auto => remaining_length.max(Points::default()) / auto_measurements as f32,
            Dimension::Points(points) => *points,
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
                Dimension::Auto => (effective_side1, effective_side2),
                Dimension::Points(_) => (
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

impl From<Surround<Dimension>> for AbsoluteBounds {
    fn from(surround: Surround<Dimension>) -> Self {
        Self {
            left: surround.left,
            right: surround.right,
            top: surround.top,
            bottom: surround.bottom,
            ..Default::default()
        }
    }
}

#[async_trait]
impl LayoutSolver for AbsoluteLayout {
    async fn layout_within(
        &self,
        bounds: &Rect<Points>,
        content_size: &Size<Points>,
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
        ($left:expr, $check:expr) => {
            assert_relative_eq!($left.0.to_f32(), $check.0);
            assert_relative_eq!($left.1.to_f32(), $check.1)
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
                Points::from_f32(90.),
                Points::from_f32(30.),
            ),
            (30., 30.)
        );

        // start.pts  end.auto length.auto
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::from_points(50.),
                &Dimension::Auto,
                &Dimension::Auto,
                Points::from_f32(90.),
                Points::from_f32(30.),
            ),
            (50., 10.)
        );

        // start.pts end.pts length.auto
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::from_points(50.),
                &Dimension::from_points(0.),
                &Dimension::Auto,
                Points::from_f32(90.),
                Points::from_f32(30.),
            ),
            (50., 0.)
        );

        // start.pts end.auto length.pts
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::from_points(10.),
                &Dimension::Auto,
                &Dimension::from_points(10.),
                Points::from_f32(90.),
                Points::from_f32(30.),
            ),
            (10., 70.)
        );

        // start.pts end.pts length.pts
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::from_points(10.),
                &Dimension::from_points(75.),
                &Dimension::from_points(5.),
                Points::from_f32(90.),
                Points::from_f32(30.),
            ),
            (10., 75.)
        );

        // start.auto end.pts length.auto
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Auto,
                &Dimension::from_points(50.),
                &Dimension::Auto,
                Points::from_f32(90.),
                Points::from_f32(30.),
            ),
            (10., 50.)
        );
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Auto,
                &Dimension::from_points(50.),
                &Dimension::Auto,
                Points::from_f32(90.),
                Points::from_f32(90.),
            ),
            (0., 50.)
        );

        // start.auto end.pts length.pts
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::Auto,
                &Dimension::from_points(50.),
                &Dimension::from_points(20.),
                Points::from_f32(90.),
                Points::from_f32(30.),
            ),
            (20., 50.)
        );

        // Running out of room: Not enough space to honor both width and padding
        // This engine's decision is to honor making the "whitespace" layout look laid out correctly
        // for as long as possible, and start clipping the content.
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::from_points(40.),
                &Dimension::from_points(40.),
                &Dimension::from_points(30.),
                Points::from_f32(90.),
                Points::from_f32(30.),
            ),
            (40., 40.)
        );

        // Running out of room: Not even enough room for padding
        assert_dimension_eq!(
            AbsoluteLayout::solve_dimension(
                &Dimension::from_points(50.),
                &Dimension::from_points(50.),
                &Dimension::from_points(30.),
                Points::from_f32(90.),
                Points::from_f32(30.),
            ),
            (45., 45.)
        );

        Ok(())
    }

    #[test]
    fn validate_tests() -> KludgineResult<()> {
        AbsoluteBounds {
            bottom: Dimension::from_points(1.),
            height: Dimension::from_points(1.),
            top: Dimension::from_points(1.),
            ..Default::default()
        }
        .validate()
        .expect_err("Invalid Vertical Bounds");
        AbsoluteBounds {
            left: Dimension::from_points(1.),
            width: Dimension::from_points(1.),
            right: Dimension::from_points(1.),
            ..Default::default()
        }
        .validate()
        .expect_err("Invalid Horizontal Bounds");

        Ok(())
    }

    #[tokio::test]
    async fn layout_test() -> KludgineResult<()> {
        use crate::{
            math::Pixels,
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
                            right: Dimension::from_points(30.),
                            bottom: Dimension::from_points(30.),
                            width: Dimension::from_points(90.),
                            height: Dimension::from_points(90.),
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
                                left: Dimension::from_points(10.),
                                top: Dimension::from_points(10.),
                                right: Dimension::from_points(10.),
                                bottom: Dimension::from_points(10.),
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

        let scene = Scene::default();
        scene
            .set_internal_size(Size::new(Pixels::from_f32(200.), Pixels::from_f32(200.)))
            .await;
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

        assert_relative_eq!(root_layout.inner_bounds().origin.x.to_f32(), 0.);
        assert_relative_eq!(root_layout.inner_bounds().origin.y.to_f32(), 0.);
        assert_relative_eq!(root_layout.inner_bounds().size.width.to_f32(), 200.);
        assert_relative_eq!(root_layout.inner_bounds().size.height.to_f32(), 200.);

        assert_relative_eq!(child_layout.inner_bounds().origin.x.to_f32(), 80.);
        assert_relative_eq!(child_layout.inner_bounds().origin.y.to_f32(), 80.);
        assert_relative_eq!(child_layout.inner_bounds().size.width.to_f32(), 90.);
        assert_relative_eq!(child_layout.inner_bounds().size.height.to_f32(), 90.);

        assert_relative_eq!(leaf_layout.inner_bounds().origin.x.to_f32(), 90.);
        assert_relative_eq!(leaf_layout.inner_bounds().origin.y.to_f32(), 90.);
        assert_relative_eq!(leaf_layout.inner_bounds().size.width.to_f32(), 70.);
        assert_relative_eq!(leaf_layout.inner_bounds().size.height.to_f32(), 70.);

        Ok(())
    }
}
