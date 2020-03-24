#[derive(Clone, Debug)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Color {
    pub fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl Into<color_processing::Color> for Color {
    fn into(self) -> color_processing::Color {
        color_processing::Color::new_rgba(self.red, self.green, self.blue, self.alpha)
    }
}
