mod batch;
mod circle;
mod fill;
mod geometry;
mod path;
mod stroke;

pub use self::{batch::*, fill::*, path::*, stroke::*};
use crate::{
    math::{Point, Raw, Rect, Scaled},
    scene::{Element, Scene},
    KludgineResult,
};
use circle::Circle;
use euclid::{Length, Scale};
use geometry::ShapeGeometry;

#[derive(Default, Clone, Debug)]
pub struct Shape<S> {
    geometry: ShapeGeometry<S>,
    stroke: Option<Stroke>,
    fill: Option<Fill>,
}

impl<S> Shape<S>
where
    S: Copy + Default,
{
    pub fn rect(rect: impl Into<Rect<f32, S>>) -> Self {
        let rect = rect.into();
        let path = PathBuilder::new(Point::new(rect.min_x(), rect.min_y()))
            .line_to(Point::new(rect.max_x(), rect.min_y()))
            .line_to(Point::new(rect.max_x(), rect.max_y()))
            .line_to(Point::new(rect.min_x(), rect.max_y()))
            .close()
            .build();

        Self {
            geometry: ShapeGeometry::Path(path),
            stroke: None,
            fill: None,
        }
    }

    pub fn circle(center: Point<f32, S>, radius: Length<f32, S>) -> Self {
        Self {
            geometry: ShapeGeometry::Circle(Circle { center, radius }),
            stroke: None,
            fill: None,
        }
    }

    pub fn polygon(points: impl IntoIterator<Item = Point<f32, S>>) -> Self {
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
}

impl Shape<Scaled> {
    pub async fn render_at(&self, location: Point<f32, Scaled>, scene: &Scene) {
        let translated = self.convert_from_user_to_device(location, scene).await;
        scene.push_element(Element::Shape(translated)).await
    }

    async fn convert_from_user_to_device(
        &self,
        location: Point<f32, Scaled>,
        scene: &Scene,
    ) -> Shape<Raw> {
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

impl Shape<Raw> {
    pub(crate) fn build(&self, builder: &mut easygpu_lyon::ShapeBuilder) -> KludgineResult<()> {
        self.geometry.build(builder, &self.stroke, &self.fill)
    }
}

impl<Src, Dst> std::ops::Mul<Scale<f32, Src, Dst>> for Shape<Src> {
    type Output = Shape<Dst>;
    fn mul(self, scale: Scale<f32, Src, Dst>) -> Self::Output {
        Self::Output {
            geometry: self.geometry * scale,
            fill: self.fill,
            stroke: self.stroke,
        }
    }
}
