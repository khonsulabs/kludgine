use crate::{math::Raw, shape::Shape, KludgineResult};
use easygpu::prelude::*;
use easygpu_lyon::ShapeBuilder;

#[derive(Default, Clone)]
pub struct Batch {
    shapes: Vec<Shape<Raw>>,
}

impl Batch {
    pub fn add(&mut self, shape: Shape<Raw>) {
        self.shapes.push(shape)
    }

    pub(crate) fn finish(self, renderer: &Renderer) -> KludgineResult<easygpu_lyon::Shape> {
        let mut builder = ShapeBuilder::default();

        for shape in self.shapes {
            shape.build(&mut builder)?;
        }

        Ok(builder.prepare(renderer))
    }
}
