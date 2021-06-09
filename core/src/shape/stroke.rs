use euclid::Length;
use lyon_tessellation::StrokeOptions;

use crate::{color::Color, math::Scaled};

#[derive(Default, Clone, Debug)]
pub struct Stroke {
    pub color: Color,
    pub options: StrokeOptions,
}

impl Stroke {
    #[must_use]
    pub fn new(color: Color) -> Self {
        Self {
            color,
            options: StrokeOptions::default(),
        }
    }

    #[must_use]
    pub const fn with_options(mut self, options: StrokeOptions) -> Self {
        self.options = options;
        self
    }

    #[must_use]
    pub fn line_width<F: Into<Length<f32, Scaled>>>(mut self, width: F) -> Self {
        self.options.line_width = width.into().get();
        self
    }
}
