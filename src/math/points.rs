use approx::relative_eq;

#[derive(Copy, Clone, PartialOrd, PartialEq, Debug, Default)]
pub struct Points(pub f32);

impl Points {
    pub fn to_f32(&self) -> f32 {
        self.0
    }

    pub fn max(&self, other: Points) -> Self {
        if relative_eq!(self.0, other.0) || self < &other {
            other
        } else {
            *self
        }
    }

    pub fn min(&self, other: Points) -> Self {
        if relative_eq!(self.0, other.0) || self > &other {
            other
        } else {
            *self
        }
    }
}

impl From<f32> for Points {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl Into<f32> for Points {
    fn into(self) -> f32 {
        self.0
    }
}

impl From<u32> for Points {
    fn from(value: u32) -> Self {
        Self(value as f32)
    }
}

impl Into<u32> for Points {
    fn into(self) -> u32 {
        self.0 as u32
    }
}

impl std::ops::Mul<Points> for Points {
    type Output = Self;

    fn mul(self, s: Points) -> Self {
        self * s.0
    }
}

impl std::ops::Mul<f32> for Points {
    type Output = Self;

    fn mul(self, s: f32) -> Self {
        Self(self.0 * s)
    }
}

impl std::ops::Div<Points> for Points {
    type Output = Self;

    fn div(self, s: Points) -> Self {
        self / s.0
    }
}

impl std::ops::Div<f32> for Points {
    type Output = Self;

    fn div(self, s: f32) -> Self {
        Self(self.0 / s)
    }
}

impl std::ops::Add<Points> for Points {
    type Output = Self;

    fn add(self, s: Points) -> Self {
        self + s.0
    }
}

impl std::ops::Add<f32> for Points {
    type Output = Self;

    fn add(self, s: f32) -> Self {
        Self(self.0 + s)
    }
}

impl std::ops::Sub<Points> for Points {
    type Output = Self;

    fn sub(self, s: Points) -> Self {
        self - s.0
    }
}

impl std::ops::Sub<f32> for Points {
    type Output = Self;

    fn sub(self, s: f32) -> Self {
        Self(self.0 - s)
    }
}

impl std::ops::Neg for Points {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl std::ops::MulAssign<f32> for Points {
    fn mul_assign(&mut self, rhs: f32) {
        self.0 *= rhs;
    }
}

impl std::ops::MulAssign<Points> for Points {
    fn mul_assign(&mut self, rhs: Points) {
        self.0 *= rhs.0;
    }
}

impl std::ops::DivAssign<f32> for Points {
    fn div_assign(&mut self, rhs: f32) {
        self.0 /= rhs;
    }
}

impl std::ops::DivAssign<Points> for Points {
    fn div_assign(&mut self, rhs: Points) {
        self.0 /= rhs.0;
    }
}

impl std::ops::SubAssign<f32> for Points {
    fn sub_assign(&mut self, rhs: f32) {
        self.0 -= rhs;
    }
}

impl std::ops::SubAssign<Points> for Points {
    fn sub_assign(&mut self, rhs: Points) {
        self.0 -= rhs.0;
    }
}

impl std::ops::AddAssign<f32> for Points {
    fn add_assign(&mut self, rhs: f32) {
        self.0 += rhs;
    }
}

impl std::ops::AddAssign<Points> for Points {
    fn add_assign(&mut self, rhs: Points) {
        self.0 += rhs.0;
    }
}

impl std::iter::Sum for Points {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut out = Self::default();
        for value in iter {
            out += value;
        }
        out
    }
}
