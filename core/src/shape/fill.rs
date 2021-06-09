use lyon_tessellation::FillOptions;

use crate::color::Color;

/// Shape fill options.
#[derive(Default, Clone, Debug)]
pub struct Fill {
    /// The color to fill.
    pub color: Color,
    /// The options to use while filling.
    pub options: FillOptions,
}

impl Fill {
    /// Returns a solid fill of `color` with default options.
    #[must_use]
    pub fn new(color: Color) -> Self {
        Self {
            color,
            options: FillOptions::default(),
        }
    }
}
