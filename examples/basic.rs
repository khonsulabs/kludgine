use std::time::Duration;

use appit::winit::error::EventLoopError;
use kludgine::figures::units::Lp;
use kludgine::figures::{Angle, Point, Rect, Size};
use kludgine::shapes::Shape;
use kludgine::text::{Text, TextOrigin};
use kludgine::Color;

const RED_SQUARE_SIZE: Lp = Lp::inches(1);

fn main() -> Result<(), EventLoopError> {
    let mut angle = Angle::degrees(0);
    kludgine::app::run(move |mut renderer, mut window| {
        window.redraw_in(Duration::from_millis(16));
        angle += Angle::degrees(180) * window.elapsed().as_secs_f32();
        let shape_center = Point::new(RED_SQUARE_SIZE / 2, RED_SQUARE_SIZE / 2);
        renderer.draw_shape(
            &Shape::filled_rect(
                Rect::<Lp>::new(
                    Point::new(-RED_SQUARE_SIZE / 2, -RED_SQUARE_SIZE / 2),
                    Size::new(RED_SQUARE_SIZE, RED_SQUARE_SIZE),
                ),
                Color::RED,
            ),
            shape_center,
            Some(angle),
            None,
        );
        renderer.draw_text(
            Text::new("Hello, World!", Color::WHITE).origin(TextOrigin::Center),
            shape_center,
            Some(angle),
            None,
        );
        true
    })
}
