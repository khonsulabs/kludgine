use crate::theme::{ColorGroup, Palette, Theme};

#[derive(Debug, Default)]
pub struct Minimal {
    palette: Palette,
}

impl Minimal {
    pub fn new(palette: Palette) -> Self {
        Self { palette }
    }
}

impl Theme for Minimal {
    fn default_font_family(&self) -> &'_ str {
        "Roboto"
    }

    fn light_control(&self) -> ColorGroup {
        self.palette.light.control.clone()
    }
}
