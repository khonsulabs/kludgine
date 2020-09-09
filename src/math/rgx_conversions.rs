use crate::math::{Raw, Rect};
use rgx::rect::Rect as RgxRect;
pub(crate) fn rect<S>(rect: &Rect<S, Raw>) -> RgxRect<S>
where
    S: Copy + std::ops::Add<S, Output = S>,
{
    let min = rect.min();
    let max = rect.max();
    RgxRect {
        x1: min.x,
        y1: min.y,
        x2: max.x,
        y2: max.y,
    }
}
