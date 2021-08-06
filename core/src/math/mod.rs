mod length_traits;
mod screen_traits;

pub use self::{length_traits::*, screen_traits::PixelAlignment};

/// A type representing a width and height.
pub type Size<T = f32, Unit = Unknown> = euclid::Size2D<T, Unit>;
/// A type representing an x and y coordinate.
pub type Point<T = f32, Unit = Unknown> = euclid::Point2D<T, Unit>;
/// A type representing a [`Point`] and [`Size`].
pub type Rect<T = f32, Unit = Unknown> = euclid::Rect<T, Unit>;
/// A measurement of length using [`Raw`] as the unit.
pub type Pixels = euclid::Length<f32, Raw>;
/// A measurement of length using [`Scaled`] as the unit.
pub type Points = euclid::Length<f32, Scaled>;
/// A type representing a vector with magnitudes x and y.
pub type Vector<T = f32, Unit = Unknown> = euclid::Vector2D<T, Unit>;
pub use euclid::{Box2D, Length, Scale};
/// The scale used to convert between [`Points`] ([`Scaled`]) and [`Pixels`]
/// ([`Raw`]).
pub type ScreenScale = Scale<f32, Scaled, Raw>;
/// A type representing an angle of measurement.
pub type Angle = euclid::Angle<f32>;

/// A unit representing physical pixels on a display.
#[derive(Debug, Clone, Copy, Default)]
pub struct Raw;

/// A unit representing [Desktop publishing points/PostScript points](https://en.wikipedia.org/wiki/Point_(typography)#Desktop_publishing_point). Measurements in this scale are equal to 1/72 of an [imperial inch](https://en.wikipedia.org/wiki/Inch).
#[derive(Debug, Clone, Copy, Default)]
pub struct Scaled;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A unit representing
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Unknown;
