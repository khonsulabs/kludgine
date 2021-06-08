use crate::{
    math::{Point, Raw, Scale, Scaled},
    scene::Target,
    shape::{circle::Circle, Fill, Path, Stroke},
    KludgineResult,
};

#[derive(Clone, Debug)]
pub(crate) enum ShapeGeometry<S> {
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

impl ShapeGeometry<Raw> {
    pub fn build(
        &self,
        builder: &mut easygpu_lyon::ShapeBuilder,
        stroke: &Option<Stroke>,
        fill: &Option<Fill>,
    ) -> KludgineResult<()> {
        match self {
            Self::Empty => Ok(()),
            Self::Path(path) => path.build(builder, stroke, fill),
            Self::Circle(circle) => circle.build(builder, stroke, fill),
        }
    }
}

impl ShapeGeometry<Scaled> {
    pub(crate) fn translate_and_convert_to_device(
        &self,
        location: Point<f32, Scaled>,
        scene: &Target,
    ) -> ShapeGeometry<Raw> {
        match self {
            Self::Empty => ShapeGeometry::Empty,
            Self::Path(path) =>
                ShapeGeometry::Path(path.translate_and_convert_to_device(location, scene)),
            Self::Circle(circle) =>
                ShapeGeometry::Circle(circle.translate_and_convert_to_device(location, scene)),
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
