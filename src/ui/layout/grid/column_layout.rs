use super::chain_layout::ChainLayout;
use crate::{
    math::{Dimension, Points, Rect, Scaled, Size, SizeExt, Surround},
    ui::{Layout, LayoutContext, LayoutSolver},
    KludgineResult,
};
use async_trait::async_trait;
use generational_arena::Index;

#[derive(Debug, Default)]
pub struct ColumnLayout {
    chain: ChainLayout,
}

impl ColumnLayout {
    pub fn column<I: Into<Index>>(mut self, child: I, width: Dimension) -> Self {
        self.chain = self.chain.element(child, width);
        self
    }

    pub fn for_each_laid_out_column<F: FnMut(Index, Layout)>(
        &self,
        bounds: &Rect<f32, Scaled>,
        mut callback: F,
    ) {
        self.chain
            .for_each_laid_out_element(bounds.size.width(), |index, x, width| {
                callback(
                    index,
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
            });
    }
}

#[async_trait]
impl LayoutSolver for ColumnLayout {
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
    fn two_auto_columns_one_fixed_smaller() {
        let mut layouts = Vec::new();
        ColumnLayout::default()
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
}
