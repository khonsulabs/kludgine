use std::time::Duration;

use appit::winit::error::EventLoopError;
use kludgine::figures::units::Lp;
use kludgine::figures::{Angle, Point, Rect, Size};
use kludgine::shapes::{PathBuilder, Shape};
use kludgine::text::{Text, TextOrigin};
use kludgine::{Color, DrawableExt};

const RED_SQUARE_SIZE: Lp = Lp::inches(1);

fn main() -> Result<(), EventLoopError> {
    let mut angle = Angle::degrees(0);
    kludgine::app::run(move |mut renderer, mut window| {
        window.redraw_in(Duration::from_millis(16));
        angle += Angle::degrees(30) * window.elapsed().as_secs_f32();
        let shape_center = Point::squared(RED_SQUARE_SIZE);
        renderer.draw_shape(
            (&Shape::filled_rect(
                Rect::<Lp>::new(
                    Point::squared(-RED_SQUARE_SIZE / 2),
                    Size::squared(RED_SQUARE_SIZE),
                ),
                Color::RED,
            ))
                .translate_by(shape_center)
                .rotate_by(angle),
        );
        renderer.draw_text(
            Text::new("Hello, World!", Color::WHITE)
                .origin(TextOrigin::Center)
                .translate_by(shape_center)
                .rotate_by(angle),
        );

        renderer.draw_shape(
            PathBuilder::new((Point::new(Lp::ZERO, Lp::inches(-1)), Color::RED))
                .line_to((Point::new(Lp::inches(1), Lp::inches(1)), Color::GREEN))
                .line_to((Point::new(Lp::inches(-1), Lp::inches(1)), Color::BLUE))
                .close()
                .filled()
                .translate_by(Point::squared(Lp::inches(3)))
                .rotate_by(-angle),
        );

        true
    })
}
