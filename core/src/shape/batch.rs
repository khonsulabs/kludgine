use easygpu::prelude::*;
use easygpu_lyon::ShapeBuilder;

use crate::{math::Raw, shape::Shape};

/// A batch of shapes that can be rendered together.
#[derive(Debug, Default, Clone)]
pub struct Batch {
    shapes: Vec<Shape<Raw>>,
}

impl Batch {
    pub(crate) fn add(&mut self, shape: Shape<Raw>) {
        self.shapes.push(shape)
    }

    pub(crate) fn finish(self, renderer: &Renderer) -> crate::Result<easygpu_lyon::Shape> {
        let mut builder = ShapeBuilder::default();

        for shape in self.shapes {
            shape.build(&mut builder)?;
        }

        Ok(builder.prepare(renderer))
    }
}
