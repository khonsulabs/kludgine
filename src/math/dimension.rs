#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::math::{Length, Scaled};

#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Dimension<Unit = Scaled> {
    Auto,
    /// In situations where applicable, attempt to shrink to fit the "content"
    /// this dimension is restricting. In all other situations, equivalent to
    /// Auto.
    Minimal,
    /// Scale-corrected to the users preference of DPI
    Length(Length<f32, Unit>),
}

impl<Unit> Dimension<Unit> {
    pub fn from_f32(value: f32) -> Self {
        Self::Length(Length::new(value))
    }

    pub fn from_length<V: Into<Length<f32, Unit>>>(value: V) -> Self {
        Self::Length(value.into())
    }

    pub fn is_auto(&self) -> bool {
        match self {
            Dimension::Minimal | Dimension::Auto => true,
            Dimension::Length(_) => false,
        }
    }

    pub fn is_length(&self) -> bool {
        !self.is_auto()
    }

    pub fn length(&self) -> Option<Length<f32, Unit>> {
        if let Dimension::Length(points) = &self {
            Some(*points)
        } else {
            None
        }
    }
}

impl<Unit> Default for Dimension<Unit> {
    fn default() -> Self {
        Dimension::Auto
    }
}

impl<Unit> From<Length<f32, Unit>> for Dimension<Unit> {
    fn from(value: Length<f32, Unit>) -> Self {
        Dimension::from_length(value)
    }
}
