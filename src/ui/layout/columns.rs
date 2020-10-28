use super::{Layout, LayoutSolver};
use crate::{
    math::{Dimension, Points, Rect, Scaled, Size, SizeExt, Surround},
    ui::LayoutContext,
    KludgineResult,
};
use async_trait::async_trait;
use generational_arena::Index;

#[derive(Debug, Default)]
pub struct ColumnarLayout {
    columns: Vec<Column>,
}

impl ColumnarLayout {
    pub fn column<I: Into<Index>>(mut self, child: I, width: Dimension) -> Self {
        self.columns.push(Column::new(child.into(), width));
        self
    }

    pub fn for_each_laid_out_column<F: FnMut(Index, Layout)>(
        &self,
        bounds: &Rect<f32, Scaled>,
        mut callback: F,
    ) {
        let mut automatic_measurements = 0usize;
        let mut defined_width = Points::default();

        for column in self.columns.iter() {
            match column.width {
                Dimension::Auto => automatic_measurements += 1,
                Dimension::Length(points) => defined_width += points,
            }
        }

        let remaining_width = (bounds.size.width() - defined_width).max(Points::default());
        let automatic_width = remaining_width / automatic_measurements as f32;

        let mut x = Points::default();
        for column in self.columns.iter() {
            let remaining_width = bounds.size.width() - x;
            let width = column
                .width
                .length()
                .unwrap_or(automatic_width)
                .min(remaining_width);

            callback(
                column.index,
                Layout {
                    bounds: *bounds,
                    margin: Surround {
                        left: x,
                        top: Points::default(),
                        right: bounds.size.width() - width - x,
                        bottom: Points::default(),
                    },
                    padding: Default::default(),
                },
            );
            x += width;
        }
    }
}

#[derive(Debug)]
struct Column {
    pub index: Index,
    pub width: Dimension,
}

impl Column {
    pub fn new(index: Index, width: Dimension) -> Self {
        Self { index, width }
    }
}

#[async_trait]
impl LayoutSolver for ColumnarLayout {
    async fn layout_within(
        &self,
        bounds: &Rect<f32, Scaled>,
        _content_size: &Size<f32, Scaled>,
        context: &LayoutContext,
    ) -> KludgineResult<()> {
        let mut layouts = Vec::new();
        self.for_each_laid_out_column(bounds, |child, layout| {
            layouts.push(context.insert_layout(child, layout));
        });
        futures::future::join_all(layouts).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Point, Rect, Size};

    #[test]
    fn one_auto_column() {
        let mut layouts = Vec::new();
        ColumnarLayout::default()
            .column(Index::from_raw_parts(0, 0), Dimension::Auto)
            .for_each_laid_out_column(
                &Rect::new(Point::new(5., 5.), Size::new(100., 150.)),
                |_, layout| {
                    layouts.push(layout);
                },
            );

        assert_eq!(layouts.len(), 1);
        assert_eq!(
            layouts[0].inner_bounds().to_u32(),
            Rect::new(Point::new(5, 5), Size::new(100, 150))
        );
    }

    #[test]
    fn two_auto_columns() {
        let mut layouts = Vec::new();
        ColumnarLayout::default()
            .column(Index::from_raw_parts(0, 0), Dimension::Auto)
            .column(Index::from_raw_parts(0, 1), Dimension::Auto)
            .for_each_laid_out_column(
                &Rect::new(Point::new(5., 5.), Size::new(100., 150.)),
                |_, layout| {
                    layouts.push(layout);
                },
            );

        assert_eq!(layouts.len(), 2);
        assert_eq!(
            layouts[0].inner_bounds().to_u32(),
            Rect::new(Point::new(5, 5), Size::new(50, 150))
        );
        assert_eq!(
            layouts[1].inner_bounds().to_u32(),
            Rect::new(Point::new(55, 5), Size::new(50, 150))
        );
    }

    #[test]
    fn two_auto_columns_one_fixed_smaller() {
        let mut layouts = Vec::new();
        ColumnarLayout::default()
            .column(Index::from_raw_parts(0, 0), Dimension::Auto)
            .column(Index::from_raw_parts(0, 1), Dimension::from_f32(30.))
            .column(Index::from_raw_parts(0, 2), Dimension::Auto)
            .for_each_laid_out_column(
                &Rect::new(Point::new(5., 5.), Size::new(100., 150.)),
                |_, layout| {
                    layouts.push(layout);
                },
            );

        assert_eq!(layouts.len(), 3);
        assert_eq!(
            layouts[0].inner_bounds().to_u32(),
            Rect::new(Point::new(5, 5), Size::new(35, 150))
        );
        assert_eq!(
            layouts[1].inner_bounds().to_u32(),
            Rect::new(Point::new(40, 5), Size::new(30, 150))
        );
        assert_eq!(
            layouts[2].inner_bounds().to_u32(),
            Rect::new(Point::new(70, 5), Size::new(35, 150))
        );
    }

    #[test]
    fn two_auto_columns_one_fixed_larger() {
        let mut layouts = Vec::new();
        ColumnarLayout::default()
            .column(Index::from_raw_parts(0, 0), Dimension::Auto)
            .column(Index::from_raw_parts(0, 1), Dimension::from_f32(70.))
            .column(Index::from_raw_parts(0, 2), Dimension::Auto)
            .for_each_laid_out_column(
                &Rect::new(Point::new(5., 5.), Size::new(100., 150.)),
                |_, layout| {
                    layouts.push(layout);
                },
            );

        assert_eq!(layouts.len(), 3);
        assert_eq!(
            layouts[0].inner_bounds().to_u32(),
            Rect::new(Point::new(5, 5), Size::new(15, 150))
        );
        assert_eq!(
            layouts[1].inner_bounds().to_u32(),
            Rect::new(Point::new(20, 5), Size::new(70, 150))
        );
        assert_eq!(
            layouts[2].inner_bounds().to_u32(),
            Rect::new(Point::new(90, 5), Size::new(15, 150))
        );
    }

    #[test]
    fn too_big() {
        let mut layouts = Vec::new();
        ColumnarLayout::default()
            .column(Index::from_raw_parts(0, 0), Dimension::Auto)
            .column(Index::from_raw_parts(0, 1), Dimension::from_f32(45.))
            .column(Index::from_raw_parts(0, 2), Dimension::from_f32(45.))
            .column(Index::from_raw_parts(0, 3), Dimension::Auto)
            .for_each_laid_out_column(
                &Rect::new(Point::new(5., 5.), Size::new(50., 150.)),
                |_, layout| {
                    layouts.push(layout);
                },
            );

        assert_eq!(layouts.len(), 4);
        assert_eq!(
            layouts[0].inner_bounds().to_u32(),
            Rect::new(Point::new(5, 5), Size::new(0, 150))
        );
        assert_eq!(
            layouts[1].inner_bounds().to_u32(),
            Rect::new(Point::new(5, 5), Size::new(45, 150))
        );
        assert_eq!(
            layouts[2].inner_bounds().to_u32(),
            Rect::new(Point::new(50, 5), Size::new(5, 150))
        );
        assert_eq!(
            layouts[3].inner_bounds().to_u32(),
            Rect::new(Point::new(55, 5), Size::new(0, 150))
        );
    }
}
