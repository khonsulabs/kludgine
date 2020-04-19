use super::Font;
use lazy_static::lazy_static;

macro_rules! include_font {
    ($path:expr) => {{
        let bytes = std::include_bytes!($path);
        Font::try_from_bytes(bytes as &[u8]).expect("Error loading bundled font")
    }};
}
lazy_static! {
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO: Font = include_font!("../../fonts/roboto/Roboto-Regular.ttf");
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_ITALIC: Font = include_font!("../../fonts/roboto/Roboto-Italic.ttf");
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_BLACK: Font = include_font!("../../fonts/roboto/Roboto-Black.ttf");
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_BLACK_ITALIC: Font = include_font!("../../fonts/roboto/Roboto-BlackItalic.ttf");
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_BOLD: Font = include_font!("../../fonts/roboto/Roboto-Bold.ttf");
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_BOLD_ITALIC: Font = include_font!("../../fonts/roboto/Roboto-BoldItalic.ttf");
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_LIGHT: Font = include_font!("../../fonts/roboto/Roboto-Light.ttf");
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_LIGHT_ITALIC: Font = include_font!("../../fonts/roboto/Roboto-LightItalic.ttf");
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_MEDIUM: Font = include_font!("../../fonts/roboto/Roboto-Medium.ttf");
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_MEDIUM_ITALIC: Font = include_font!("../../fonts/roboto/Roboto-MediumItalic.ttf");
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_THIN: Font = include_font!("../../fonts/roboto/Roboto-Thin.ttf");
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_THIN_ITALIC: Font = include_font!("../../fonts/roboto/Roboto-ThinItalic.ttf");
}
