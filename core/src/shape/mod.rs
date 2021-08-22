mod batch;
mod circle;
mod fill;
mod geometry;
mod path;
mod stroke;

use circle::Circle;
use easygpu_lyon::lyon_tessellation;
use figures::{Displayable, Figure, Points, Rectlike, Scale};
use geometry::ShapeGeometry;

pub use self::{batch::*, fill::*, path::*, stroke::*};
use crate::{
    math::{Pixels, Point, Rect, Scaled},
    scene::{Element, Target},
};

/// A 2d shape.
#[derive(Debug, Clone)]
pub struct Shape<Unit> {
    geometry: ShapeGeometry<Unit>,
    stroke: Option<Stroke>,
    fill: Option<Fill>,
}

impl<Unit> Default for Shape<Unit> {
    fn default() -> Self {
        Self {
            geometry: ShapeGeometry::Empty,
            stroke: None,
            fill: None,
        }
    }
}

impl<Unit> Shape<Unit> {
    /// Returns a rectangle.
    pub fn rect(rect: impl Into<Rect<f32, Unit>>) -> Self {
        let rect = rect.into().as_extents();
        let path = PathBuilder::new(Point::from_figures(rect.origin.x(), rect.origin.y()))
            .line_to(Point::from_figures(rect.extent.x(), rect.origin.y()))
            .line_to(Point::from_figures(rect.extent.x(), rect.extent.y()))
            .line_to(Point::from_figures(rect.origin.x(), rect.extent.y()))
            .close()
            .build();

        Self {
            geometry: ShapeGeometry::Path(path),
            stroke: None,
            fill: None,
        }
    }

    /// Returns a circle with `center` and `radius`.
    #[must_use]
    pub const fn circle(center: Point<f32, Unit>, radius: Figure<f32, Unit>) -> Self {
        Self {
            geometry: ShapeGeometry::Circle(Circle { center, radius }),
            stroke: None,
            fill: None,
        }
    }

    /// Returns a closed polygon created with `points`.
    #[must_use]
    pub fn polygon(points: impl IntoIterator<Item = Point<f32, Unit>>) -> Self {
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

    /// Builder-style function. Set `fill` and returns self.
    #[must_use]
    pub const fn fill(mut self, fill: Fill) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Builder-style function. Set `stroke` and returns self.
    #[must_use]
    pub const fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }

    /// Returns the shape with the geometry casted to the unit provided. This
    /// does not change the underlying shape data at all.
    #[must_use]
    pub fn cast_unit<U>(self) -> Shape<U> {
        Shape {
            geometry: self.geometry.cast_unit(),
            fill: self.fill,
            stroke: self.stroke,
        }
    }
}

impl<Unit> Shape<Unit>
where
    Self: Displayable<f32, Pixels = Shape<Pixels>>,
{
    /// Renders the shape within `scene`. Uses the coordinates in the shape
    /// without translation.
    pub fn render(&self, scene: &Target) {
        self.render_at(&Point::<f32, Pixels>::default(), scene);
    }

    /// Renders the shape at `location` within `scene`.
    pub fn render_at(
        &self,
        location: &impl Displayable<f32, Pixels = Point<f32, Pixels>>,
        scene: &Target,
    ) {
        let location = location.to_pixels(scene.scale());
        let pixel_shape = self.to_pixels(scene.scale());
        let translated = pixel_shape.convert_from_user_to_device(location, scene);
        scene.push_element(Element::Shape(translated));
    }
}

impl Shape<Pixels> {
    fn convert_from_user_to_device(&self, location: Point<f32, Pixels>, scene: &Target) -> Self {
        Self {
            geometry: self
                .geometry
                .translate_and_convert_to_device(location, scene),
            fill: self.fill.clone(),
            stroke: self.stroke.clone(),
        }
    }
}

impl Shape<Pixels> {
    pub(crate) fn build(&self, builder: &mut easygpu_lyon::ShapeBuilder) -> crate::Result<()> {
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

impl Displayable<f32> for Shape<Pixels> {
    type Pixels = Self;
    type Points = Shape<Points>;
    type Scaled = Shape<Scaled>;

    fn to_pixels(&self, _scale: &figures::DisplayScale<f32>) -> Self::Pixels {
        self.clone()
    }

    fn to_points(&self, scale: &figures::DisplayScale<f32>) -> Self::Points {
        Shape {
            geometry: self.geometry.to_points(scale),
            stroke: self.stroke.clone(),
            fill: self.fill.clone(),
        }
    }

    fn to_scaled(&self, scale: &figures::DisplayScale<f32>) -> Self::Scaled {
        Shape {
            geometry: self.geometry.to_scaled(scale),
            stroke: self.stroke.clone(),
            fill: self.fill.clone(),
        }
    }
}

impl Displayable<f32> for Shape<Points> {
    type Pixels = Shape<Pixels>;
    type Points = Self;
    type Scaled = Shape<Scaled>;

    fn to_pixels(&self, scale: &figures::DisplayScale<f32>) -> Self::Pixels {
        Shape {
            geometry: self.geometry.to_pixels(scale),
            stroke: self.stroke.clone(),
            fill: self.fill.clone(),
        }
    }

    fn to_points(&self, _scale: &figures::DisplayScale<f32>) -> Self::Points {
        self.clone()
    }

    fn to_scaled(&self, scale: &figures::DisplayScale<f32>) -> Self::Scaled {
        Shape {
            geometry: self.geometry.to_scaled(scale),
            stroke: self.stroke.clone(),
            fill: self.fill.clone(),
        }
    }
}

impl Displayable<f32> for Shape<Scaled> {
    type Pixels = Shape<Pixels>;
    type Points = Shape<Points>;
    type Scaled = Self;

    fn to_pixels(&self, scale: &figures::DisplayScale<f32>) -> Self::Pixels {
        Shape {
            geometry: self.geometry.to_pixels(scale),
            stroke: self.stroke.clone(),
            fill: self.fill.clone(),
        }
    }

    fn to_points(&self, scale: &figures::DisplayScale<f32>) -> Self::Points {
        Shape {
            geometry: self.geometry.to_points(scale),
            stroke: self.stroke.clone(),
            fill: self.fill.clone(),
        }
    }

    fn to_scaled(&self, _scale: &figures::DisplayScale<f32>) -> Self::Scaled {
        self.clone()
    }
}

const fn lyon_point<T>(pt: Point<f32, T>) -> lyon_tessellation::math::Point {
    lyon_tessellation::math::Point::new(pt.x, pt.y)
}
