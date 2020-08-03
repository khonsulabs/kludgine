use approx::relative_eq;

#[derive(Copy, Clone, PartialOrd, PartialEq, Debug, Default)]
pub struct Pixels(pub f32);

impl Pixels {
    pub fn to_f32(&self) -> f32 {
        self.0
    }

    pub fn max(&self, other: Self) -> Self {
        if relative_eq!(self.0, other.0) || self < &other {
            other
        } else {
            *self
        }
    }

    pub fn min(&self, other: Self) -> Self {
        if relative_eq!(self.0, other.0) || self > &other {
            other
        } else {
            *self
        }
    }
}

impl From<u32> for Pixels {
    fn from(value: u32) -> Self {
        Self(value as f32)
    }
}

impl Into<u32> for Pixels {
    fn into(self) -> u32 {
        self.0 as u32
    }
}

impl From<f32> for Pixels {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl Into<f32> for Pixels {
    fn into(self) -> f32 {
        self.0
    }
}

impl std::ops::Mul<Pixels> for Pixels {
    type Output = Self;

    fn mul(self, s: Pixels) -> Self {
        self * s.0
    }
}

impl std::ops::Mul<f32> for Pixels {
    type Output = Self;

    fn mul(self, s: f32) -> Self {
        Self(self.0 * s)
    }
}

impl std::ops::Div<Pixels> for Pixels {
    type Output = Self;

    fn div(self, s: Pixels) -> Self {
        self / s.0
    }
}

impl std::ops::Div<f32> for Pixels {
    type Output = Self;

    fn div(self, s: f32) -> Self {
        Self(self.0 / s)
    }
}

impl std::ops::Add<Pixels> for Pixels {
    type Output = Self;

    fn add(self, s: Pixels) -> Self {
        self + s.0
    }
}

impl std::ops::Add<f32> for Pixels {
    type Output = Self;

    fn add(self, s: f32) -> Self {
        Self(self.0 + s)
    }
}

impl std::ops::Sub<Pixels> for Pixels {
    type Output = Self;

    fn sub(self, s: Pixels) -> Self {
        self - s.0
    }
}

impl std::ops::Sub<f32> for Pixels {
    type Output = Self;

    fn sub(self, s: f32) -> Self {
        Self(self.0 - s)
    }
}

impl std::ops::Neg for Pixels {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl std::ops::MulAssign<f32> for Pixels {
    fn mul_assign(&mut self, rhs: f32) {
        self.0 *= rhs;
    }
}

impl std::ops::MulAssign<Pixels> for Pixels {
    fn mul_assign(&mut self, rhs: Pixels) {
        self.0 *= rhs.0;
    }
}

impl std::ops::DivAssign<f32> for Pixels {
    fn div_assign(&mut self, rhs: f32) {
        self.0 /= rhs;
    }
}

impl std::ops::DivAssign<Pixels> for Pixels {
    fn div_assign(&mut self, rhs: Pixels) {
        self.0 /= rhs.0;
    }
}

impl std::ops::SubAssign<f32> for Pixels {
    fn sub_assign(&mut self, rhs: f32) {
        self.0 -= rhs;
    }
}

impl std::ops::SubAssign<Pixels> for Pixels {
    fn sub_assign(&mut self, rhs: Pixels) {
        self.0 -= rhs.0;
    }
}

impl std::ops::AddAssign<f32> for Pixels {
    fn add_assign(&mut self, rhs: f32) {
        self.0 += rhs;
    }
}

impl std::ops::AddAssign<Pixels> for Pixels {
    fn add_assign(&mut self, rhs: Pixels) {
        self.0 += rhs.0;
    }
}

impl std::iter::Sum for Pixels {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut out = Self::default();
        for value in iter {
            out += value;
        }
        out
    }
}
