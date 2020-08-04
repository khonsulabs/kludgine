use crate::math::Points;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Dimension {
    Auto,
    /// Scale-corrected to the users preference of DPI
    Points(Points),
}

impl Dimension {
    pub fn from_points(value: impl Into<Points>) -> Self {
        Self::Points(value.into())
    }

    pub fn is_auto(&self) -> bool {
        self == &Dimension::Auto
    }
    pub fn is_points(&self) -> bool {
        !self.is_auto()
    }

    pub fn points(&self) -> Option<Points> {
        if let Dimension::Points(points) = &self {
            Some(*points)
        } else {
            None
        }
    }
}

impl Default for Dimension {
    fn default() -> Self {
        Dimension::Auto
    }
}

impl From<Points> for Dimension {
    fn from(value: Points) -> Self {
        Dimension::from_points(value)
    }
}
