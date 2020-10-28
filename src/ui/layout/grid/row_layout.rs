use super::chain_layout::{ChainElementContents, ChainElementDimensionTranslator, ChainLayout};
use crate::math::{Dimension, Points, Scaled, Size, SizeExt, Surround};
use std::ops::Deref;

#[derive(Debug, Default)]
pub struct RowLayout {
    chain: ChainLayout,
}

impl ChainElementDimensionTranslator for RowLayout {
    fn convert_to_margin(min: Points, max: Points) -> Surround<f32, Scaled> {
        Surround {
            left: Points::default(),
            top: min,
            right: Points::default(),
            bottom: max,
        }
    }

    fn length_from_size(size: &Size<f32, Scaled>) -> Points {
        size.height()
    }

    fn size_replacing_length(size: &Size<f32, Scaled>, length: Points) -> Size<f32, Scaled> {
        let mut size = *size;
        size.set_height(length);
        size
    }
}

impl RowLayout {
    pub fn row<I: Into<ChainElementContents>>(mut self, child: I, height: Dimension) -> Self {
        self.chain = self.chain.element(child, height);
        self
    }
}

impl Deref for RowLayout {
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
    fn two_auto_rows_one_fixed_smaller() {
        let (_, layouts) = RowLayout::default()
            .row(Index::from_raw_parts(0, 0), Dimension::Auto)
            .row(Index::from_raw_parts(0, 1), Dimension::from_f32(30.))
            .row(Index::from_raw_parts(0, 2), Dimension::Auto)
            .layouts_within_bounds(
                &Rect::new(Point::new(5., 5.), Size::new(150., 100.)),
                &HashMap::default(),
            );

        assert_eq!(layouts.len(), 3);
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 0)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 5), Size::new(150, 35))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 1)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 40), Size::new(150, 30))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 2)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 70), Size::new(150, 35))
        );
    }

    #[test]
    fn one_auto_column() {
        let (_, layouts) = RowLayout::default()
            .row(Index::from_raw_parts(0, 0), Dimension::Auto)
            .layouts_within_bounds(
                &Rect::new(Point::new(5., 5.), Size::new(150., 100.)),
                &HashMap::default(),
            );

        assert_eq!(layouts.len(), 1);
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 0)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 5), Size::new(150, 100))
        );
    }

    #[test]
    fn one_auto_one_minimal_one_fixed() {
        let (_, layouts) = RowLayout::default()
            .row(Index::from_raw_parts(0, 0), Dimension::Auto)
            .row(Index::from_raw_parts(0, 1), Dimension::Minimal)
            .row(Index::from_raw_parts(0, 2), Dimension::from_f32(20.))
            .layouts_within_bounds(
                &Rect::new(Point::new(5., 5.), Size::new(150., 100.)),
                &hash_map!(Index::from_raw_parts(0, 1) => Size::new(150., 10.)),
            );

        assert_eq!(layouts.len(), 3);
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 0)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 5), Size::new(150, 70))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 1)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 75), Size::new(150, 10))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 2)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 85), Size::new(150, 20))
        );
    }

    #[test]
    fn two_auto_columns() {
        let (_, layouts) = RowLayout::default()
            .row(Index::from_raw_parts(0, 0), Dimension::Auto)
            .row(Index::from_raw_parts(0, 1), Dimension::Auto)
            .layouts_within_bounds(
                &Rect::new(Point::new(5., 5.), Size::new(150., 100.)),
                &HashMap::default(),
            );

        assert_eq!(layouts.len(), 2);
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 0)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 5), Size::new(150, 50))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 1)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 55), Size::new(150, 50))
        );
    }

    #[test]
    fn two_auto_columns_one_fixed_smaller() {
        let (_, layouts) = RowLayout::default()
            .row(Index::from_raw_parts(0, 0), Dimension::Auto)
            .row(Index::from_raw_parts(0, 1), Dimension::from_f32(30.))
            .row(Index::from_raw_parts(0, 2), Dimension::Auto)
            .layouts_within_bounds(
                &Rect::new(Point::new(5., 5.), Size::new(150., 100.)),
                &HashMap::default(),
            );

        assert_eq!(layouts.len(), 3);
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 0)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 5), Size::new(150, 35))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 1)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 40), Size::new(150, 30))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 2)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 70), Size::new(150, 35))
        );
    }

    #[test]
    fn two_auto_columns_one_fixed_larger() {
        let (_, layouts) = RowLayout::default()
            .row(Index::from_raw_parts(0, 0), Dimension::Auto)
            .row(Index::from_raw_parts(0, 1), Dimension::from_f32(70.))
            .row(Index::from_raw_parts(0, 2), Dimension::Auto)
            .layouts_within_bounds(
                &Rect::new(Point::new(5., 5.), Size::new(150., 100.)),
                &HashMap::default(),
            );

        assert_eq!(layouts.len(), 3);
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 0)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 5), Size::new(150, 15))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 1)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 20), Size::new(150, 70))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 2)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 90), Size::new(150, 15))
        );
    }

    #[test]
    fn too_big() {
        let (_, layouts) = RowLayout::default()
            .row(Index::from_raw_parts(0, 0), Dimension::Auto)
            .row(Index::from_raw_parts(0, 1), Dimension::from_f32(45.))
            .row(Index::from_raw_parts(0, 2), Dimension::from_f32(45.))
            .row(Index::from_raw_parts(0, 3), Dimension::Auto)
            .layouts_within_bounds(
                &Rect::new(Point::new(5., 5.), Size::new(150., 50.)),
                &HashMap::default(),
            );

        assert_eq!(layouts.len(), 4);
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 0)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 5), Size::new(150, 0))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 1)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 5), Size::new(150, 45))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 2)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 50), Size::new(150, 5))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 3)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 55), Size::new(150, 0))
        );
    }
}
