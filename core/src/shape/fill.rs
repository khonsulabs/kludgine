use lyon_tessellation::FillOptions;

use crate::color::Color;

#[derive(Default, Clone, Debug)]
pub struct Fill {
    pub color: Color,
    pub options: FillOptions,
}

impl Fill {
    #[must_use]
    pub fn new(color: Color) -> Self {
        Self {
            color,
            options: FillOptions::default(),
        }
    }
}
