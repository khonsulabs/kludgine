use crate::{math::Raw, shape::Shape, KludgineResult};
use rgx_lyon::ShapeBuilder;

#[derive(Default, Clone)]
pub struct Batch {
    shapes: Vec<Shape<Raw>>,
}

impl Batch {
    pub fn add(&mut self, shape: Shape<Raw>) {
        self.shapes.push(shape)
    }

    pub(crate) fn finish(self, renderer: &rgx::core::Renderer) -> KludgineResult<rgx_lyon::Shape> {
        let mut builder = ShapeBuilder::default();

        for shape in self.shapes {
            shape.build(&mut builder)?;
        }

        Ok(builder.prepare(renderer))
    }
}