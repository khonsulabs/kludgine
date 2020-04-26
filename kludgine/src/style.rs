use crate::{
    math::{Dimension, Point, Rect, Size, Surround},
    scene::SceneTarget,
};
pub use rgx::color::Rgba as Color;
pub use ttf_parser::Weight;

#[derive(Default, Clone, Debug)]
pub struct Layout {
    pub location: Point,
    pub margin: Surround<Dimension>,
    pub padding: Surround<Dimension>,
    pub border: Surround<Dimension>,
    pub min_size: Size<Dimension>,
    pub max_size: Size<Dimension>,
}

impl Layout {
    pub fn size_with_minimal_padding(&self, size: &Size) -> Size {
        Size::new(
            size.width - self.padding.minimum_width(),
            size.height - self.padding.minimum_height(),
        )
    }

    pub fn compute_bounds(&self, content_size: &Size, bounds: Rect) -> Rect {
        let (effective_padding_left, effective_padding_right) = Self::compute_padding(
            self.padding.left,
            self.padding.right,
            content_size.width,
            bounds.size.width,
        );
        let (effective_padding_top, effective_padding_bottom) = Self::compute_padding(
            self.padding.top,
            self.padding.bottom,
            content_size.height,
            bounds.size.height,
        );
        Rect::new(
            Point::new(
                bounds.x1() + effective_padding_left,
                bounds.y1() + effective_padding_top,
            ),
            Point::new(
                bounds.x2() - effective_padding_right,
                bounds.y2() - effective_padding_bottom,
            ),
        )
    }

    pub fn compute_padding(
        side1: Dimension,
        side2: Dimension,
        content_measurement: f32,
        bounding_measurement: f32,
    ) -> (f32, f32) {
        let mut remaining_width = bounding_measurement - content_measurement;
        let mut auto_width_measurements = 0;
        if let Some(points) = side1.points() {
            remaining_width -= points;
        } else {
            auto_width_measurements += 1;
        }

        if let Some(points) = side2.points() {
            remaining_width -= points;
        } else {
            auto_width_measurements += 1;
        }

        let effective_side1 = match side1 {
            Dimension::Auto => remaining_width / auto_width_measurements as f32,
            Dimension::Points(points) => points,
        };

        let effective_side2 = match side2 {
            Dimension::Auto => remaining_width / auto_width_measurements as f32,
            Dimension::Points(points) => points,
        };

        (effective_side1, effective_side2)
    }
}

#[derive(Default, Clone, Debug)]
pub struct Style {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: Option<Weight>,
    pub color: Option<Color>,
}

impl Style {
    pub fn inherit_from(&self, parent: &Style) -> Self {
        Self {
            font_family: self.font_family.clone().or(parent.font_family.clone()),
            font_size: self.font_size.or(parent.font_size),
            font_weight: self.font_weight.or(parent.font_weight),
            color: self.color.or(parent.color),
        }
    }

    pub fn effective_style(&self, scene: &mut SceneTarget) -> EffectiveStyle {
        EffectiveStyle {
            font_family: self
                .font_family
                .clone()
                .unwrap_or_else(|| "sans-serif".to_owned()),
            font_size: self.font_size.unwrap_or(14.0) * scene.effective_scale_factor(),
            font_weight: self.font_weight.unwrap_or(Weight::Normal),
            color: self.color.unwrap_or(Color::BLACK),
        }
    }
}

#[derive(PartialEq, Clone, Debug, Default)]
pub struct EffectiveStyle {
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: Weight,
    pub color: Color,
}
