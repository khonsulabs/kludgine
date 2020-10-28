use crate::math::{Dimension, Points};
use generational_arena::Index;

#[derive(Debug, Default)]
pub(crate) struct ChainLayout {
    elements: Vec<ChainElement>,
}

impl ChainLayout {
    pub fn element<I: Into<Index>>(mut self, child: I, height: Dimension) -> Self {
        self.elements.push(ChainElement::new(child.into(), height));
        self
    }

    pub fn for_each_laid_out_element<F: FnMut(Index, Points, Points)>(
        &self,
        full_size: Points,
        mut callback: F,
    ) {
        let mut automatic_measurements = 0usize;
        let mut defined_size = Points::default();

        for element in self.elements.iter() {
            match element.width {
                Dimension::Auto => automatic_measurements += 1,
                Dimension::Length(points) => defined_size += points,
            }
        }

        let remaining_size = (full_size - defined_size).max(Points::default());
        let automatic_size = remaining_size / automatic_measurements as f32;

        let mut x = Points::default();
        for column in self.elements.iter() {
            let remaining_size = full_size - x;
            let size = column
                .width
                .length()
                .unwrap_or(automatic_size)
                .min(remaining_size);

            callback(column.index, x, size);
            x += size;
        }
    }
}

#[derive(Debug)]
struct ChainElement {
    pub index: Index,
    pub width: Dimension,
}

impl ChainElement {
    pub fn new(index: Index, width: Dimension) -> Self {
        Self { index, width }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_auto_column() {
        let mut layouts = Vec::new();
        ChainLayout::default()
            .element(Index::from_raw_parts(0, 0), Dimension::Auto)
            .for_each_laid_out_element(Points::new(100.), |_, pos, length| {
                layouts.push((pos.get() as u32, length.get() as u32));
            });

        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0], (0, 100));
    }

    #[test]
    fn two_auto_columns() {
        let mut layouts = Vec::new();
        ChainLayout::default()
            .element(Index::from_raw_parts(0, 0), Dimension::Auto)
            .element(Index::from_raw_parts(0, 1), Dimension::Auto)
            .for_each_laid_out_element(Points::new(100.), |_, pos, length| {
                layouts.push((pos.get() as u32, length.get() as u32));
            });

        assert_eq!(layouts.len(), 2);
        assert_eq!(layouts[0], (0, 50));
        assert_eq!(layouts[1], (50, 50));
    }

    #[test]
    fn two_auto_columns_one_fixed_smaller() {
        let mut layouts = Vec::new();
        ChainLayout::default()
            .element(Index::from_raw_parts(0, 0), Dimension::Auto)
            .element(Index::from_raw_parts(0, 1), Dimension::from_f32(30.))
            .element(Index::from_raw_parts(0, 2), Dimension::Auto)
            .for_each_laid_out_element(Points::new(100.), |_, pos, length| {
                layouts.push((pos.get() as u32, length.get() as u32));
            });

        assert_eq!(layouts.len(), 3);
        assert_eq!(layouts[0], (0, 35));
        assert_eq!(layouts[1], (35, 30));
        assert_eq!(layouts[2], (65, 35));
    }

    #[test]
    fn two_auto_columns_one_fixed_larger() {
        let mut layouts = Vec::new();
        ChainLayout::default()
            .element(Index::from_raw_parts(0, 0), Dimension::Auto)
            .element(Index::from_raw_parts(0, 1), Dimension::from_f32(70.))
            .element(Index::from_raw_parts(0, 2), Dimension::Auto)
            .for_each_laid_out_element(Points::new(100.), |_, pos, length| {
                layouts.push((pos.get() as u32, length.get() as u32));
            });

        assert_eq!(layouts.len(), 3);
        assert_eq!(layouts[0], (0, 15));
        assert_eq!(layouts[1], (15, 70));
        assert_eq!(layouts[2], (85, 15));
    }

    #[test]
    fn too_big() {
        let mut layouts = Vec::new();
        ChainLayout::default()
            .element(Index::from_raw_parts(0, 0), Dimension::Auto)
            .element(Index::from_raw_parts(0, 1), Dimension::from_f32(45.))
            .element(Index::from_raw_parts(0, 2), Dimension::from_f32(45.))
            .element(Index::from_raw_parts(0, 3), Dimension::Auto)
            .for_each_laid_out_element(Points::new(50.), |_, pos, length| {
                layouts.push((pos.get() as u32, length.get() as u32));
            });

        assert_eq!(layouts.len(), 4);
        assert_eq!(layouts[0], (0, 0));
        assert_eq!(layouts[1], (0, 45));
        assert_eq!(layouts[2], (45, 5));
        assert_eq!(layouts[3], (50, 0));
    }
}
