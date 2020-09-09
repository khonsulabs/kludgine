use crate::color::Color;

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
