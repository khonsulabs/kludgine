use crate::{
    math::{Raw, Scaled},
    style::{GenericStyle, Style, StyleComponent},
};

pub trait FallbackStyle<Unit>: Sized {
    fn lookup(style: &Style<Unit>) -> Option<Self>;
}

pub trait UnscaledFallbackStyle: StyleComponent<Scaled> + Clone {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style.get::<Self>().cloned()
    }
}

impl<T> FallbackStyle<Scaled> for T
where
    T: UnscaledFallbackStyle,
{
    fn lookup(style: &Style<Scaled>) -> Option<Self> {
        T::lookup_unscaled(GenericStyle::Scaled(style))
    }
}

impl<T> FallbackStyle<Raw> for T
where
    T: UnscaledFallbackStyle,
{
    fn lookup(style: &Style<Raw>) -> Option<Self> {
        T::lookup_unscaled(GenericStyle::Raw(style))
    }
}
