mod batch;
mod path;

pub use self::{batch::*, path::*};
use crate::{
    color::Color,
    math::{Pixels, Point, Points, Rect},
    scene::{Element, SceneTarget},
};

#[derive(Default, Clone)]
pub struct Shape<S> {
    path: Path<S>,
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
            path,
            stroke: None,
            fill: None,
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
            path: self
                .path
                .translate_and_convert_to_device(location, scene)
                .await,
            fill: self.fill.clone(),
            stroke: self.stroke.clone(),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct Fill {
    pub color: Color,
    pub options: lyon_tessellation::FillOptions,
}

impl Fill {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            options: Default::default(),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct Stroke {
    pub color: Color,
    pub options: lyon_tessellation::StrokeOptions,
}

impl Stroke {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            options: Default::default(),
        }
    }
}
