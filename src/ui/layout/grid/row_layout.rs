use super::chain_layout::ChainLayout;
use crate::{
    math::{Dimension, Points, Rect, Scaled, Size, SizeExt, Surround},
    ui::{Layout, LayoutContext, LayoutSolver},
    KludgineResult,
};
use async_trait::async_trait;
use generational_arena::Index;

#[derive(Debug, Default)]
pub struct RowLayout {
    chain: ChainLayout,
}

impl RowLayout {
    pub fn row<I: Into<Index>>(mut self, child: I, height: Dimension) -> Self {
        self.chain = self.chain.element(child, height);
        self
    }

    pub fn for_each_laid_out_row<F: FnMut(Index, Layout)>(
        &self,
        bounds: &Rect<f32, Scaled>,
        mut callback: F,
    ) {
        self.chain
            .for_each_laid_out_element(bounds.size.height(), |index, y, height| {
                callback(
                    index,
                    Layout {
                        bounds: *bounds,
                        margin: Surround {
                            left: Points::default(),
                            top: y,
                            right: Points::default(),
                            bottom: bounds.size.height() - height - y,
                        },
                        padding: Default::default(),
                    },
                );
            });
    }
}

#[async_trait]
impl LayoutSolver for RowLayout {
    async fn layout_within(
        &self,
        bounds: &Rect<f32, Scaled>,
        _content_size: &Size<f32, Scaled>,
        context: &LayoutContext,
    ) -> KludgineResult<()> {
        let mut layouts = Vec::new();
        self.for_each_laid_out_row(bounds, |child, layout| {
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
    fn two_auto_rows_one_fixed_smaller() {
        let mut layouts = Vec::new();
        RowLayout::default()
            .row(Index::from_raw_parts(0, 0), Dimension::Auto)
            .row(Index::from_raw_parts(0, 1), Dimension::from_f32(30.))
            .row(Index::from_raw_parts(0, 2), Dimension::Auto)
            .for_each_laid_out_row(
                &Rect::new(Point::new(5., 5.), Size::new(150., 100.)),
                |_, layout| {
                    layouts.push(layout);
                },
            );

        assert_eq!(layouts.len(), 3);
        assert_eq!(
            layouts[0].inner_bounds().to_u32(),
            Rect::new(Point::new(5, 5), Size::new(150, 35))
        );
        assert_eq!(
            layouts[1].inner_bounds().to_u32(),
            Rect::new(Point::new(5, 40), Size::new(150, 30))
        );
        assert_eq!(
            layouts[2].inner_bounds().to_u32(),
            Rect::new(Point::new(5, 70), Size::new(150, 35))
        );
    }
}
