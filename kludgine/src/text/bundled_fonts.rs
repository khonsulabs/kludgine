//! Feature-flag enabled fonts that are licensed under [APL 2.0](https://github.com/khonsulabs/kludgine/blob/master/fonts/LICENSE.txt)
//!
//! See [fonts/README.md](https://github.com/khonsulabs/kludgine/blob/master/fonts/README.md) for more information on licensing
//!
//! To enable all bundled fonts, enable the `bundled-fonts` cargo feature:
//!
//! ```toml
//! [dependencies]
//! kludgine = { version = ..., features = ["bundled-fonts"] }
//! ```
//!
//! To enable a single font, look at the documentation of the font in question.
//!
//! WHen enabled, the Scene object's font library is initialized with all bundled fonts that are enabled.

use super::Font;
use lazy_static::lazy_static;

macro_rules! include_font {
    ($path:expr) => {{
        let bytes = std::include_bytes!($path);
        Font::try_from_bytes(bytes as &[u8]).expect("Error loading bundled font")
    }};
}

lazy_static! {
    /// Roboto Regular font, licensed under APL 2.0, feature flag `bundled-fonts-roboto`
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO: Font = include_font!("../../fonts/roboto/Roboto-Regular.ttf");
    /// Roboto Italic font, licensed under APL 2.0, feature flag `bundled-fonts-roboto`
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_ITALIC: Font = include_font!("../../fonts/roboto/Roboto-Italic.ttf");
    /// Roboto Black font, licensed under APL 2.0, feature flag `bundled-fonts-roboto`
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_BLACK: Font = include_font!("../../fonts/roboto/Roboto-Black.ttf");
    /// Roboto Black-Italic font, licensed under APL 2.0, feature flag `bundled-fonts-roboto`
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_BLACK_ITALIC: Font = include_font!("../../fonts/roboto/Roboto-BlackItalic.ttf");
    /// Roboto Bold font, licensed under APL 2.0, feature flag `bundled-fonts-roboto`
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_BOLD: Font = include_font!("../../fonts/roboto/Roboto-Bold.ttf");
    /// Roboto Bold-Italic font, licensed under APL 2.0, feature flag `bundled-fonts-roboto`
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_BOLD_ITALIC: Font = include_font!("../../fonts/roboto/Roboto-BoldItalic.ttf");
    /// Roboto Light font, licensed under APL 2.0, feature flag `bundled-fonts-roboto`
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_LIGHT: Font = include_font!("../../fonts/roboto/Roboto-Light.ttf");
    /// Roboto Light-Italic font, licensed under APL 2.0, feature flag `bundled-fonts-roboto`
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_LIGHT_ITALIC: Font = include_font!("../../fonts/roboto/Roboto-LightItalic.ttf");
    /// Roboto Medium font, licensed under APL 2.0, feature flag `bundled-fonts-roboto`
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_MEDIUM: Font = include_font!("../../fonts/roboto/Roboto-Medium.ttf");
    /// Roboto Medium-Italic font, licensed under APL 2.0, feature flag `bundled-fonts-roboto`
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_MEDIUM_ITALIC: Font = include_font!("../../fonts/roboto/Roboto-MediumItalic.ttf");
    /// Roboto  font, licensed under APL 2.0, feature flag `bundled-fonts-roboto`
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_THIN: Font = include_font!("../../fonts/roboto/Roboto-Thin.ttf");
    /// Roboto Thin-Italic font, licensed under APL 2.0, feature flag `bundled-fonts-roboto`
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO_THIN_ITALIC: Font = include_font!("../../fonts/roboto/Roboto-ThinItalic.ttf");
}
