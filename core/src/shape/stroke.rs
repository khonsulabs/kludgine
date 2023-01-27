use easygpu_lyon::lyon_tessellation::StrokeOptions;
use figures::Figure;

use crate::color::Color;
use crate::math::Scaled;

/// A shape stroke (outline) options.
#[derive(Default, Clone, Debug)]
pub struct Stroke {
    /// The color to stroke the shape's with.
    pub color: Color,
    /// The options for drawing the stroke.
    pub options: StrokeOptions,
}

impl Stroke {
    /// Creates a new instance using `color` with default options.
    #[must_use]
    pub fn new(color: Color) -> Self {
        Self {
            color,
            options: StrokeOptions::default(),
        }
    }

    /// Builder-style function. Sets `options` and return self.
    #[must_use]
    pub const fn with_options(mut self, options: StrokeOptions) -> Self {
        self.options = options;
        self
    }

    /// Builder-style function. Sets `options.line_width` and return self.
    #[must_use]
    pub fn line_width<F: Into<Figure<f32, Scaled>>>(mut self, width: F) -> Self {
        self.options.line_width = width.into().get();
        self
    }
}
