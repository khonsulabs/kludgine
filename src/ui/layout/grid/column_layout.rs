use crate::{
    math::{Dimension, Points, Scaled, Size, SizeExt, Surround},
    ui::layout::grid::chain_layout::{
        ChainElementContents, ChainElementDimensionTranslator, ChainLayout,
    },
};
use std::ops::Deref;

#[derive(Debug, Default)]
pub struct ColumnLayout {
    chain: ChainLayout,
}

impl ChainElementDimensionTranslator for ColumnLayout {
    fn convert_to_margin(min: Points, max: Points) -> Surround<f32, Scaled> {
        Surround {
            top: Points::default(),
            left: min,
            bottom: Points::default(),
            right: max,
        }
    }

    fn length_from_size(size: &Size<f32, Scaled>) -> Points {
        size.width()
    }

    fn size_replacing_length(size: &Size<f32, Scaled>, length: Points) -> Size<f32, Scaled> {
        let mut size = *size;
        size.set_width(length);
        size
    }
}

impl ColumnLayout {
    pub fn column<I: Into<ChainElementContents>>(mut self, child: I, width: Dimension) -> Self {
        self.chain = self.chain.element(child, width);
        self
    }
}

impl Deref for ColumnLayout {
    type Target = ChainLayout;

    fn deref(&self) -> &Self::Target {
        &self.chain
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        math::{Point, Rect, Size},
        ui::layout::grid::chain_layout::ChainElementDynamicContents,
    };
    use generational_arena::Index;

    #[test]
    fn two_auto_columns_one_fixed_smaller() {
        let (_, layouts) = ColumnLayout::default()
            .column(Index::from_raw_parts(0, 0), Dimension::Auto)
            .column(Index::from_raw_parts(0, 1), Dimension::from_f32(30.))
            .column(Index::from_raw_parts(0, 2), Dimension::Auto)
            .layouts_within_bounds(
                &Rect::new(Point::new(5., 5.), Size::new(100., 150.)),
                &HashMap::default(),
            );

        assert_eq!(layouts.len(), 3);
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 0)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 5), Size::new(35, 150))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 1)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(40, 5), Size::new(30, 150))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 2)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(70, 5), Size::new(35, 150))
        );
    }
}
