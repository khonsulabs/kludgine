use crate::{
    math::{Point, Raw, Scaled},
    scene::SceneTarget,
    shape::{circle::Circle, Fill, Path, Stroke},
    KludgineResult,
};

#[derive(Clone)]
pub(crate) enum ShapeGeometry<S> {
    Empty,
    Path(Path<S>),
    Circle(Circle<S>),
}

impl ShapeGeometry<Raw> {
    pub fn build(
        &self,
        builder: &mut rgx_lyon::ShapeBuilder,
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
    pub(crate) async fn translate_and_convert_to_device(
        &self,
        location: Point<f32, Scaled>,
        scene: &SceneTarget,
    ) -> ShapeGeometry<Raw> {
        match self {
            Self::Empty => ShapeGeometry::Empty,
            Self::Path(path) => {
                ShapeGeometry::Path(path.translate_and_convert_to_device(location, scene).await)
            }
            Self::Circle(circle) => ShapeGeometry::Circle(
                circle
                    .translate_and_convert_to_device(location, scene)
                    .await,
            ),
        }
    }
}

impl<S> Default for ShapeGeometry<S> {
    fn default() -> Self {
        Self::Empty
    }
}
