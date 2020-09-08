mod batch;
mod circle;
mod fill;
mod geometry;
mod path;
mod stroke;

pub use self::{batch::*, fill::*, path::*, stroke::*};
use crate::{
    math::{Pixels, Point, Points, Rect},
    scene::{Element, SceneTarget},
    KludgineResult,
};
use circle::Circle;
use geometry::ShapeGeometry;

#[derive(Default, Clone)]
pub struct Shape<S> {
    geometry: ShapeGeometry<S>,
    stroke: Option<Stroke>,
    fill: Option<Fill>,
}

impl Shape<Points> {
    pub fn rect(rect: impl Into<Rect<Points>>) -> Self {
        let rect = rect.into();
        let path = PathBuilder::new(rect.x1y1())
            .line_to(rect.x2y1())
            .line_to(rect.x2y2())
            .line_to(rect.x1y2())
            .close()
            .build();

        Self {
            geometry: ShapeGeometry::Path(path),
            stroke: None,
            fill: None,
        }
    }

    pub fn circle(center: Point<Points>, radius: Points) -> Self {
        Self {
            geometry: ShapeGeometry::Circle(Circle { center, radius }),
            stroke: None,
            fill: None,
        }
    }

    pub fn polygon(points: impl IntoIterator<Item = Point<Points>>) -> Self {
        let mut points = points.into_iter();
        if let Some(start) = points.next() {
            let mut builder = PathBuilder::new(start);
            for point in points {
                builder = builder.line_to(point);
            }

            Self {
                geometry: ShapeGeometry::Path(builder.close().build()),
                stroke: None,
                fill: None,
            }
        } else {
            Self::default()
        }
    }

    pub fn fill(mut self, fill: Fill) -> Self {
        self.fill = Some(fill);
        self
    }

    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }

    pub async fn draw_at(&self, location: Point<Points>, scene: &SceneTarget) {
        let translated = self.convert_from_user_to_device(location, scene).await;
        scene.push_element(Element::Shape(translated)).await
    }

    async fn convert_from_user_to_device(
        &self,
        location: Point<Points>,
        scene: &SceneTarget,
    ) -> Shape<Pixels> {
        Shape {
            geometry: self
                .geometry
                .translate_and_convert_to_device(location, scene)
                .await,
            fill: self.fill.clone(),
            stroke: self.stroke.clone(),
        }
    }
}

impl Shape<Pixels> {
    pub(crate) fn build(&self, builder: &mut rgx_lyon::ShapeBuilder) -> KludgineResult<()> {
        self.geometry.build(builder, &self.stroke, &self.fill)
    }
}
