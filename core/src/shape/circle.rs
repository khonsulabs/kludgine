use figures::{Displayable, Points, Vectorlike};

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

impl Circle<Pixels> {
    pub(crate) fn translate_and_convert_to_device(
        &self,
        location: Point<f32, Pixels>,
        scene: &Target,
    ) -> Self {
        let effective_scale = scene.scale();
        let center = location + self.center.to_vector();
        let radius = self.radius.to_pixels(effective_scale);
        Self { center, radius }
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

impl Displayable<f32> for Circle<Pixels> {
    type Pixels = Self;
    type Points = Circle<Points>;
    type Scaled = Circle<Scaled>;

    fn to_pixels(&self, _scale: &figures::DisplayScale<f32>) -> Self::Pixels {
        self.clone()
    }

    fn to_points(&self, scale: &figures::DisplayScale<f32>) -> Self::Points {
        Circle {
            center: self.center.to_points(scale),
            radius: self.radius.to_points(scale),
        }
    }

    fn to_scaled(&self, scale: &figures::DisplayScale<f32>) -> Self::Scaled {
        Circle {
            center: self.center.to_scaled(scale),
            radius: self.radius.to_scaled(scale),
        }
    }
}

impl Displayable<f32> for Circle<Points> {
    type Pixels = Circle<Pixels>;
    type Points = Self;
    type Scaled = Circle<Scaled>;

    fn to_pixels(&self, scale: &figures::DisplayScale<f32>) -> Self::Pixels {
        Circle {
            center: self.center.to_pixels(scale),
            radius: self.radius.to_pixels(scale),
        }
    }

    fn to_points(&self, _scale: &figures::DisplayScale<f32>) -> Self::Points {
        self.clone()
    }

    fn to_scaled(&self, scale: &figures::DisplayScale<f32>) -> Self::Scaled {
        Circle {
            center: self.center.to_scaled(scale),
            radius: self.radius.to_scaled(scale),
        }
    }
}

impl Displayable<f32> for Circle<Scaled> {
    type Pixels = Circle<Pixels>;
    type Points = Circle<Points>;
    type Scaled = Self;

    fn to_pixels(&self, scale: &figures::DisplayScale<f32>) -> Self::Pixels {
        Circle {
            center: self.center.to_pixels(scale),
            radius: self.radius.to_pixels(scale),
        }
    }

    fn to_points(&self, scale: &figures::DisplayScale<f32>) -> Self::Points {
        Circle {
            center: self.center.to_points(scale),
            radius: self.radius.to_points(scale),
        }
    }

    fn to_scaled(&self, _scale: &figures::DisplayScale<f32>) -> Self::Scaled {
        self.clone()
    }
}
