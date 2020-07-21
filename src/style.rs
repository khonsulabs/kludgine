use crate::{
    math::{Dimension, Size, Surround},
    scene::SceneTarget,
};
pub use rgx::color::Rgba as Color;
pub use stretch::style::{
    AlignContent, AlignItems, AlignSelf, Direction, Display, FlexDirection, FlexWrap,
    JustifyContent, Overflow, PositionType,
};
pub use ttf_parser::Weight;

#[derive(Default, Clone, Debug)]
pub struct Layout {
    pub display: Display,
    pub position_type: PositionType,
    pub direction: Direction,
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub overflow: Overflow,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub align_content: AlignContent,
    pub justify_content: JustifyContent,
    pub position: Surround<Dimension>,
    pub margin: Surround<Dimension>,
    pub padding: Surround<Dimension>,
    pub border: Surround<Dimension>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub size: Size<Dimension>,
    pub min_size: Size<Dimension>,
    pub max_size: Size<Dimension>,
    pub aspect_ratio: Option<f32>,
}

impl Layout {
    pub fn interior_size_with_padding(&self, size: &Size) -> Size {
        Size::new(
            size.width - self.padding.minimum_width(),
            size.height - self.padding.minimum_height(),
        )
    }
    pub fn size_with_padding(&self, size: &Size) -> Size {
        Size::new(
            size.width + self.padding.minimum_width(),
            size.height + self.padding.minimum_height(),
        )
    }

    // pub fn compute_padding(&self, content_size: &Size, bounds: &Rect) -> Surround {
    //     let (left, right) = Self::compute_padding_for_length(
    //         self.padding.left,
    //         self.padding.right,
    //         content_size.width,
    //         bounds.size.width,
    //     );
    //     let (top, bottom) = Self::compute_padding_for_length(
    //         self.padding.top,
    //         self.padding.bottom,
    //         content_size.height,
    //         bounds.size.height,
    //     );
    //     Surround {
    //         left,
    //         right,
    //         top,
    //         bottom,
    //     }
    // }

    // fn compute_padding_for_length(
    //     side1: Dimension,
    //     side2: Dimension,
    //     content_measurement: f32,
    //     bounding_measurement: f32,
    // ) -> (f32, f32) {
    //     let mut remaining_width = bounding_measurement - content_measurement;
    //     let mut auto_width_measurements = 0;
    //     if let Some(points) = side1.points() {
    //         remaining_width -= points;
    //     } else {
    //         auto_width_measurements += 1;
    //     }

    //     if let Some(points) = side2.points() {
    //         remaining_width -= points;
    //     } else {
    //         auto_width_measurements += 1;
    //     }

    //     let effective_side1 = match side1 {
    //         Dimension::Auto => remaining_width / auto_width_measurements as f32,
    //         Dimension::Points(points) => points,
    //     };

    //     let effective_side2 = match side2 {
    //         Dimension::Auto => remaining_width / auto_width_measurements as f32,
    //         Dimension::Points(points) => points,
    //     };

    //     (effective_side1, effective_side2)
    // }
}

impl Into<stretch::style::Style> for Layout {
    fn into(self) -> stretch::style::Style {
        stretch::style::Style {
            position: self.position.into(),
            padding: self.padding.into(),
            display: self.display,
            position_type: self.position_type,
            direction: self.direction,
            flex_direction: self.flex_direction,
            flex_wrap: self.flex_wrap,
            overflow: self.overflow,
            align_items: self.align_items,
            align_self: self.align_self,
            align_content: self.align_content,
            justify_content: self.justify_content,

            margin: self.margin.into(),

            border: self.border.into(),
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: self.flex_basis.into(),
            size: self.size.into(),
            min_size: self.min_size.into(),
            max_size: self.max_size.into(),
            aspect_ratio: self
                .aspect_ratio
                .map(stretch::number::Number::Defined).unwrap_or_default(),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct Style {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: Option<Weight>,
    pub color: Option<Color>,
    pub background_color: Option<Color>,
}

impl Style {
    pub fn inherit_from(&self, parent: &Style) -> Self {
        Self {
            font_family: self
                .font_family
                .clone()
                .or_else(|| parent.font_family.clone()),
            font_size: self.font_size.or(parent.font_size),
            font_weight: self.font_weight.or(parent.font_weight),
            color: self.color.or(parent.color),
            background_color: self.background_color.or(parent.background_color),
        }
    }

    pub async fn effective_style(&self, scene: &SceneTarget) -> EffectiveStyle {
        EffectiveStyle {
            font_family: self
                .font_family
                .clone()
                .unwrap_or_else(|| "sans-serif".to_owned()),
            font_size: self.font_size.unwrap_or(14.0) * scene.effective_scale_factor().await,
            font_weight: self.font_weight.unwrap_or(Weight::Normal),
            color: self.color.unwrap_or(Color::BLACK),
            background_color: self.background_color,
        }
    }
}

#[derive(PartialEq, Clone, Debug, Default)]
pub struct EffectiveStyle {
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: Weight,
    pub color: Color,
    pub background_color: Option<Color>,
}
