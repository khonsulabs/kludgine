use euclid::Length;

use crate::{color::Color, math::Scaled};

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

    pub fn with_options(mut self, options: lyon_tessellation::StrokeOptions) -> Self {
        self.options = options;
        self
    }

    pub fn line_width<F: Into<Length<f32, Scaled>>>(mut self, width: F) -> Self {
        self.options.line_width = width.into().get();
        self
    }
}
