use std::fmt::Debug;

use figures::{Displayable, Points};

use crate::{
    math::{Pixels, Point, Scale, Scaled},
    scene::Target,
    shape::{circle::Circle, Fill, Path, Stroke},
};

#[derive(Clone, Debug)]
pub enum ShapeGeometry<S> {
    Empty,
    Path(Path<S>),
    Circle(Circle<S>),
}

impl<U> ShapeGeometry<U> {
    pub fn cast_unit<V>(self) -> ShapeGeometry<V> {
        match self {
            Self::Empty => ShapeGeometry::Empty,
            Self::Path(path) => ShapeGeometry::Path(path.cast_unit()),
            Self::Circle(circle) => ShapeGeometry::Circle(circle.cast_unit()),
        }
    }
}

impl ShapeGeometry<Pixels> {
    pub fn build(
        &self,
        builder: &mut easygpu_lyon::ShapeBuilder,
        stroke: &Option<Stroke>,
        fill: &Option<Fill>,
    ) -> crate::Result<()> {
        match self {
            Self::Empty => Ok(()),
            Self::Path(path) => path.build(builder, stroke, fill),
            Self::Circle(circle) => circle.build(builder, stroke, fill),
        }
    }
}

impl ShapeGeometry<Pixels> {
    pub(crate) fn translate_and_convert_to_device(
        &self,
        location: Point<f32, Pixels>,
        scene: &Target,
    ) -> Self {
        match self {
            Self::Empty => Self::Empty,
            Self::Path(path) => Self::Path(path.translate_and_convert_to_device(location, scene)),
            Self::Circle(circle) =>
                Self::Circle(circle.translate_and_convert_to_device(location, scene)),
        }
    }
}

impl<S> Default for ShapeGeometry<S> {
    fn default() -> Self {
        Self::Empty
    }
}

impl<Src, Dst> std::ops::Mul<Scale<f32, Src, Dst>> for ShapeGeometry<Src> {
    type Output = ShapeGeometry<Dst>;

    fn mul(self, rhs: Scale<f32, Src, Dst>) -> Self::Output {
        match self {
            Self::Empty => Self::Output::Empty,
            Self::Path(path) => Self::Output::Path(path * rhs),
            Self::Circle(circle) => Self::Output::Circle(circle * rhs),
        }
    }
}

impl Displayable<f32> for ShapeGeometry<Pixels> {
    type Pixels = Self;
    type Points = ShapeGeometry<Points>;
    type Scaled = ShapeGeometry<Scaled>;

    fn to_pixels(&self, _scale: &figures::DisplayScale<f32>) -> Self::Pixels {
        self.clone()
    }

    fn to_points(&self, scale: &figures::DisplayScale<f32>) -> Self::Points {
        match self {
            Self::Empty => ShapeGeometry::Empty,
            Self::Path(path) => ShapeGeometry::Path(path.to_points(scale)),
            Self::Circle(circle) => ShapeGeometry::Circle(circle.to_points(scale)),
        }
    }

    fn to_scaled(&self, scale: &figures::DisplayScale<f32>) -> Self::Scaled {
        match self {
            Self::Empty => ShapeGeometry::Empty,
            Self::Path(path) => ShapeGeometry::Path(path.to_scaled(scale)),
            Self::Circle(circle) => ShapeGeometry::Circle(circle.to_scaled(scale)),
        }
    }
}

impl Displayable<f32> for ShapeGeometry<Points> {
    type Pixels = ShapeGeometry<Pixels>;
    type Points = Self;
    type Scaled = ShapeGeometry<Scaled>;

    fn to_pixels(&self, scale: &figures::DisplayScale<f32>) -> Self::Pixels {
        match self {
            Self::Empty => ShapeGeometry::Empty,
            Self::Path(path) => ShapeGeometry::Path(path.to_pixels(scale)),
            Self::Circle(circle) => ShapeGeometry::Circle(circle.to_pixels(scale)),
        }
    }

    fn to_points(&self, _scale: &figures::DisplayScale<f32>) -> Self::Points {
        self.clone()
    }

    fn to_scaled(&self, scale: &figures::DisplayScale<f32>) -> Self::Scaled {
        match self {
            Self::Empty => ShapeGeometry::Empty,
            Self::Path(path) => ShapeGeometry::Path(path.to_scaled(scale)),
            Self::Circle(circle) => ShapeGeometry::Circle(circle.to_scaled(scale)),
        }
    }
}

impl Displayable<f32> for ShapeGeometry<Scaled> {
    type Pixels = ShapeGeometry<Pixels>;
    type Points = ShapeGeometry<Points>;
    type Scaled = Self;

    fn to_pixels(&self, scale: &figures::DisplayScale<f32>) -> Self::Pixels {
        match self {
            Self::Empty => ShapeGeometry::Empty,
            Self::Path(path) => ShapeGeometry::Path(path.to_pixels(scale)),
            Self::Circle(circle) => ShapeGeometry::Circle(circle.to_pixels(scale)),
        }
    }

    fn to_points(&self, scale: &figures::DisplayScale<f32>) -> Self::Points {
        match self {
            Self::Empty => ShapeGeometry::Empty,
            Self::Path(path) => ShapeGeometry::Path(path.to_points(scale)),
            Self::Circle(circle) => ShapeGeometry::Circle(circle.to_points(scale)),
        }
    }

    fn to_scaled(&self, _scale: &figures::DisplayScale<f32>) -> Self::Scaled {
        self.clone()
    }
}
