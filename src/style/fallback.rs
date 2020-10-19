use crate::{
    math::{Raw, Scaled},
    style::{GenericStyle, Style},
};

pub trait FallbackStyle<Unit>: Sized {
    fn lookup(style: &Style<Unit>) -> Option<Self>;
}

pub trait UnscaledFallbackStyle: Sized {
    fn lookup(style: GenericStyle) -> Option<Self>;
}

impl<T> FallbackStyle<Scaled> for T
where
    T: UnscaledFallbackStyle,
{
    fn lookup(style: &Style<Scaled>) -> Option<Self> {
        T::lookup(GenericStyle::Scaled(style))
    }
}

impl<T> FallbackStyle<Raw> for T
where
    T: UnscaledFallbackStyle,
{
    fn lookup(style: &Style<Raw>) -> Option<Self> {
        T::lookup(GenericStyle::Raw(style))
    }
}
