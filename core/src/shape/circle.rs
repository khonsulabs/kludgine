use figures::{Displayable, Vectorlike};

use super::lyon_point;
use crate::{
    math::{Figure, Pixels, Point, Scale, Scaled},
    scene::Target,
    shape::{Fill, Stroke},
    Error,
};
#[derive(Clone, Debug)]
pub struct Circle<S> {
    pub center: Point<f32, S>,
    pub radius: Figure<f32, S>,
}

impl<U> Circle<U> {
    pub fn cast_unit<V>(self) -> Circle<V> {
        Circle {
            center: self.center.cast_unit(),
            radius: self.radius.cast_unit(),
        }
    }
}

impl Circle<Scaled> {
    pub(crate) fn translate_and_convert_to_device(
        &self,
        location: Point<f32, Scaled>,
        scene: &Target,
    ) -> Circle<Pixels> {
        let effective_scale = scene.scale();
        let center = (location + self.center.to_vector()).to_pixels(effective_scale);
        let radius = self.radius.to_pixels(effective_scale);
        Circle { center, radius }
    }
}

impl Circle<Pixels> {
    pub fn build(
        &self,
        builder: &mut easygpu_lyon::ShapeBuilder,
        stroke: &Option<Stroke>,
        fill: &Option<Fill>,
    ) -> crate::Result<()> {
        if let Some(fill) = fill {
            builder.default_color = fill.color.rgba();
            lyon_tessellation::basic_shapes::fill_circle(
                lyon_point(self.center),
                self.radius.get(),
                &fill.options,
                builder,
            )
            .map_err(Error::Tessellation)?;
        }

        if let Some(stroke) = stroke {
            builder.default_color = stroke.color.rgba();
            lyon_tessellation::basic_shapes::stroke_circle(
                lyon_point(self.center),
                self.radius.get(),
                &stroke.options,
                builder,
            )
            .map_err(Error::Tessellation)?;
        }

        Ok(())
    }
}

impl<Src, Dst> std::ops::Mul<Scale<f32, Src, Dst>> for Circle<Src> {
    type Output = Circle<Dst>;

    fn mul(self, scale: Scale<f32, Src, Dst>) -> Self::Output {
        Self::Output {
            center: self.center * scale,
            radius: self.radius * scale,
        }
    }
}
