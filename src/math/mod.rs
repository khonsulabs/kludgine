mod dimension;
mod measurement;
mod point;
mod rect;
mod size;
mod surround;

pub use self::{dimension::*, measurement::*, point::*, rect::*, size::*, surround::*};

pub(crate) fn max_f(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

pub(crate) fn min_f(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn min_max_tests() {
        assert_relative_eq!(min_f(0.0, 1.0), 0.0);
        assert_relative_eq!(min_f(1.0, 0.0), 0.0);
        assert_relative_eq!(min_f(0.0, 0.0), 0.0);

        assert_relative_eq!(max_f(0.0, 1.0), 1.0);
        assert_relative_eq!(max_f(1.0, 0.0), 1.0);
        assert_relative_eq!(max_f(0.0, 0.0), 0.0);
    }
}
