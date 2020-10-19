use crate::{
    math::{Raw, Scaled},
    style::UnscaledStyleComponent,
};
#[derive(Debug, Clone)]
pub struct FontFamily(pub String);
impl UnscaledStyleComponent<Raw> for FontFamily {}
impl UnscaledStyleComponent<Scaled> for FontFamily {}
impl Default for FontFamily {
    fn default() -> Self {
        Self("sans-serif".to_owned())
    }
}

impl<T> From<T> for FontFamily
where
    T: ToString,
{
    fn from(family: T) -> Self {
        Self(family.to_string())
    }
}
