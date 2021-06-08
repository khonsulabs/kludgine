mod dimension;
mod length_traits;
mod screen_traits;
mod surround;

pub use self::{dimension::*, length_traits::*, screen_traits::PixelAlignment, surround::*};

pub type Size<T = f32, Unit = Unknown> = euclid::Size2D<T, Unit>;
pub type Point<T = f32, Unit = Unknown> = euclid::Point2D<T, Unit>;
pub type Rect<T = f32, Unit = Unknown> = euclid::Rect<T, Unit>;
pub type Pixels = euclid::Length<f32, Raw>;
pub type Points = euclid::Length<f32, Scaled>;
pub type Vector<T = f32, Unit = Unknown> = euclid::Vector2D<T, Unit>;
pub use euclid::{Box2D, Length, Scale};
pub type ScreenScale = Scale<f32, Scaled, Raw>;
pub type Angle = euclid::Angle<f32>;

/// A unit representing physical pixels on a display.
#[derive(Debug, Clone, Copy, Default)]
pub struct Raw;

/// A unit representing [Desktop publishing points/PostScript points](https://en.wikipedia.org/wiki/Point_(typography)#Desktop_publishing_point). Measurements in this scale are equal to 1/72 of an [imperial inch](https://en.wikipedia.org/wiki/Inch).
#[derive(Debug, Clone, Copy, Default)]
pub struct Scaled;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Unknown;
