use super::Font;
use lazy_static::lazy_static;
use std::include_bytes;

lazy_static! {
    #[cfg(feature="bundled-fonts-roboto")]
    pub static ref ROBOTO: Font = {
        let bytes = include_bytes!("../../fonts/roboto/Roboto-Regular.ttf");
        Font::try_from_bytes(bytes as &[u8])
            .expect("Error loading bundled font")
    };
}
