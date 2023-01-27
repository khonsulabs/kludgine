use easygpu::prelude::*;
use easygpu_lyon::ShapeBuilder;

use crate::math::Pixels;
use crate::shape::Shape;

/// A batch of shapes that can be rendered together.
#[derive(Debug, Default, Clone)]
pub struct Batch {
    shapes: Vec<Shape<Pixels>>,
}

impl Batch {
    pub(crate) fn add(&mut self, shape: Shape<Pixels>) {
        self.shapes.push(shape);
    }

    pub(crate) fn finish(self, renderer: &Renderer) -> crate::Result<easygpu_lyon::Shape> {
        let mut builder = ShapeBuilder::default();

        for shape in self.shapes {
            shape.build(&mut builder)?;
        }

        Ok(builder.prepare(renderer))
    }
}
