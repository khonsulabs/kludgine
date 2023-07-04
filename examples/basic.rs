use std::time::Duration;

use kludgine::figures::{Dips, Point, Rect, Size};
use kludgine::shapes::Shape;
use kludgine::Color;

const RED_SQUARE_SIZE: Dips = Dips::INCH;

fn main() {
    let mut angle = 0.;
    kludgine::app::run(move |mut renderer, mut window| {
        window.redraw_in(Duration::from_millis(16));
        angle += std::f32::consts::PI / 36.;
        renderer.draw_shape(
            &Shape::filled_rect(
                Rect::<Dips>::new(
                    Point::new(-RED_SQUARE_SIZE / 2, -RED_SQUARE_SIZE / 2),
                    Size::new(RED_SQUARE_SIZE, RED_SQUARE_SIZE),
                ),
                Color::RED,
            ),
            Point::new(RED_SQUARE_SIZE / 2, RED_SQUARE_SIZE / 2),
            Some(angle),
            None,
        );
        true
    })
}
