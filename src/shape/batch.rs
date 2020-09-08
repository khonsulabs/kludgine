use crate::{math::Pixels, shape::Shape, KludgineError, KludgineResult};
use rgx_lyon::ShapeBuilder;

#[derive(Default, Clone)]
pub struct Batch {
    shapes: Vec<Shape<Pixels>>,
}

impl Batch {
    pub fn add(&mut self, shape: Shape<Pixels>) {
        self.shapes.push(shape)
    }

    pub(crate) fn finish(self, renderer: &rgx::core::Renderer) -> KludgineResult<rgx_lyon::Shape> {
        let mut builder = ShapeBuilder::default();

        for shape in self.shapes {
            if let Some(fill) = shape.fill {
                builder.default_color = fill.color.rgba();
                builder
                    .fill(&shape.path.as_lyon(), &fill.options)
                    .map_err(KludgineError::TesselationError)?;
            }

            if let Some(stroke) = shape.stroke {
                builder.default_color = stroke.color.rgba();
                builder
                    .stroke(&shape.path.as_lyon(), &stroke.options)
                    .map_err(KludgineError::TesselationError)?;
            }
        }

        Ok(builder.prepare(renderer))
    }
}
