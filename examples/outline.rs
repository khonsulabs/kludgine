//! This example shows pixel-perfect drawing of shapes at the extremes of the
//! window and a clipping rect. If any portion of the 1-pixel wide line isn't
//! visible, this would be a bug.
use appit::winit::error::EventLoopError;
use kludgine::figures::units::UPx;
use kludgine::figures::{Point, Rect};
use kludgine::shapes::{Shape, StrokeOptions};
use kludgine::Color;

fn main() -> Result<(), EventLoopError> {
    kludgine::app::run(move |mut renderer, _window| {
        let visible_rect = Rect::from(renderer.size() - Point::new(UPx(1), UPx(1)));
        renderer.draw_shape(&Shape::stroked_rect(
            visible_rect,
            Color::RED,
            StrokeOptions::default(),
        ));

        let mut clipped = renderer.clipped_to(visible_rect.inset(100));
        let visible_rect = Rect::from(clipped.size() - Point::new(UPx(1), UPx(1)));
        clipped.draw_shape(&Shape::stroked_rect(
            visible_rect,
            Color::BLUE,
            StrokeOptions::default(),
        ));

        true
    })
}
