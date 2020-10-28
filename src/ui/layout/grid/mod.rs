mod chain_layout;
mod column_layout;
mod row_layout;

pub use self::{column_layout::ColumnLayout, row_layout::RowLayout};

#[cfg(test)]
mod tests {
    use generational_arena::Index;

    use super::*;
    use crate::{
        math::{Dimension, Point, Rect, Size},
        ui::layout::grid::chain_layout::ChainElementDynamicContents,
    };

    #[test]
    fn full_grid() {
        let layouts = ColumnLayout::default()
            .column(
                RowLayout::default()
                    .row(Index::from_raw_parts(0, 0), Dimension::Auto)
                    .row(Index::from_raw_parts(0, 1), Dimension::from_f32(70.))
                    .row(Index::from_raw_parts(0, 2), Dimension::Auto),
                Dimension::Auto,
            )
            .column(Index::from_raw_parts(1, 0), Dimension::from_f32(30.))
            .column(Index::from_raw_parts(2, 0), Dimension::Auto)
            .layouts_within_bounds(&Rect::new(Point::new(5., 5.), Size::new(100., 150.)));

        assert_eq!(layouts.len(), 5);
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 0)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 5), Size::new(35, 40))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 1)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 45), Size::new(35, 70))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(0, 2)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(5, 115), Size::new(35, 40))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(1, 0)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(40, 5), Size::new(30, 150))
        );
        assert_eq!(
            layouts[&Index::from_raw_parts(2, 0)]
                .inner_bounds()
                .to_u32(),
            Rect::new(Point::new(70, 5), Size::new(35, 150))
        );
    }
}
