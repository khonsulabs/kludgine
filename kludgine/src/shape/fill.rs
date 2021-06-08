use crate::color::Color;

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
