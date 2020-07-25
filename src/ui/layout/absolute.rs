use crate::{
    math::{Dimension, Point, Rect, Size, Surround},
    ui::{
        global_arena,
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
}

impl AbsoluteLayout {
    pub fn child(
        mut self,
        index: impl Into<Index>,
        bounds: AbsoluteBounds,
    ) -> KludgineResult<Self> {
        self.children.insert(index.into(), bounds.validate()?);
        Ok(self)
    }

    fn solve_dimension(
        start: &Dimension,
        end: &Dimension,
        available_length: f32,
        content_length: f32,
    ) -> (f32, f32, f32) {
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
            Dimension::Auto => remaining_length / auto_measurements as f32,
            Dimension::Points(points) => *points,
        };

        let effective_side2 = match end {
            Dimension::Auto => remaining_length / auto_measurements as f32,
            Dimension::Points(points) => *points,
        };

        // TODO Shrink length if sides are too large. Make sure s1 + l + s2 = content_length

        (effective_side1, content_length, effective_side2)
    }

    // pub fn compute_padding(&self, content_size: &Size, bounds: &Rect) -> Surround {
    //     let (left, right) = Self::compute_padding_for_length(
    //         self.padding.left,
    //         self.padding.right,
    //         content_size.width,
    //         bounds.size.width,
    //     );
    //     let (top, bottom) = Self::compute_padding_for_length(
    //         self.padding.top,
    //         self.padding.bottom,
    //         content_size.height,
    //         bounds.size.height,
    //     );
    //     Surround {
    //         left,
    //         right,
    //         top,
    //         bottom,
    //     }
    // }

    // fn compute_padding_for_length(
    //     side1: Dimension,
    //     side2: Dimension,
    //     content_measurement: f32,
    //     bounding_measurement: f32,
    // ) -> (f32, f32) {
    //     let mut remaining_width = bounding_measurement - content_measurement;
    //     let mut auto_width_measurements = 0;
    //     if let Some(points) = side1.points() {
    //         remaining_width -= points;
    //     } else {
    //         auto_width_measurements += 1;
    //     }

    //     if let Some(points) = side2.points() {
    //         remaining_width -= points;
    //     } else {
    //         auto_width_measurements += 1;
    //     }

    //     let effective_side1 = match side1 {
    //         Dimension::Auto => remaining_width / auto_width_measurements as f32,
    //         Dimension::Points(points) => points,
    //     };

    //     let effective_side2 = match side2 {
    //         Dimension::Auto => remaining_width / auto_width_measurements as f32,
    //         Dimension::Points(points) => points,
    //     };

    //     (effective_side1, effective_side2)
    // }
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
    ) -> KludgineResult<HashMap<Index, Layout>> {
        println!("Absolute Layout solving for {:?}", content_size);
        let mut computed_layouts = HashMap::new();
        for (&index, child_bounds) in self.children.iter() {
            let mut child_context = context.clone_for(index).await;
            let child_content_size = global_arena()
                .get(index)
                .await
                .unwrap()
                .content_size(
                    child_context.styled_context(),
                    &Size::new(Some(content_size.width), Some(content_size.height)),
                )
                .await?;
            println!("Child content size: {:?}", child_content_size);
            let (left, width, right) = Self::solve_dimension(
                &child_bounds.left,
                &child_bounds.right,
                bounds.size.width,
                child_bounds
                    .width
                    .points()
                    .unwrap_or(child_content_size.width),
            );
            let (top, height, bottom) = Self::solve_dimension(
                &child_bounds.top,
                &child_bounds.bottom,
                bounds.size.height,
                child_bounds
                    .height
                    .points()
                    .unwrap_or(child_content_size.height),
            );

            computed_layouts.insert(
                index,
                Layout {
                    bounds: Rect::sized(
                        bounds.origin + Point::new(left, top),
                        Size::new(width, height),
                    ),
                    padding: Surround {
                        left,
                        top,
                        right,
                        bottom,
                    },
                },
            );
        }

        Ok(computed_layouts)
    }
}
