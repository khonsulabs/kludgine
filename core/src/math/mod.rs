mod screen_traits;

pub use self::screen_traits::PixelAlignment;

/// A type representing a width and height.
pub type Size<T = f32, Unit = Unknown> = figures::Size<T, Unit>;
/// A type representing an x and y coordinate.
pub type Point<T = f32, Unit = Unknown> = figures::Point<T, Unit>;
/// A type representing a [`Point`] and [`Size`].
pub type Rect<T = f32, Unit = Unknown> = figures::SizedRect<T, Unit>;
/// A measurement of length using [`Raw`] as the unit.
pub type Pixels = figures::Figure<f32, Raw>;
/// A measurement of length using [`Scaled`] as the unit.
pub type Points = figures::Figure<f32, Scaled>;
/// A type representing a vector with magnitudes x and y.
pub type Vector<T = f32, Unit = Unknown> = figures::Vector<T, Unit>;
pub use figures::{ExtentsRect, Figure, Scale};
/// The scale used to convert between [`Points`] ([`Scaled`]) and [`Pixels`]
/// ([`Raw`]).
pub type ScreenScale = Scale<f32, Scaled, Raw>;
/// A type representing an angle of measurement.
pub type Angle = figures::Angle<f32>;

/// A unit representing physical pixels on a display.
// #[derive(Debug, Clone, Copy, Default)]
// pub struct Raw;
pub type Raw = figures::Pixels;

/// A unit representing [Desktop publishing points/PostScript points](https://en.wikipedia.org/wiki/Point_(typography)#Desktop_publishing_point). Measurements in this scale are equal to 1/72 of an [imperial inch](https://en.wikipedia.org/wiki/Inch).
// #[derive(Debug, Clone, Copy, Default)]
// pub struct Scaled;
pub type Scaled = figures::Scaled;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A unit representing
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Unknown;
