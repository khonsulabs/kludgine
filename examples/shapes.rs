use std::time::Duration;

use appit::winit::error::EventLoopError;
use figures::{Pow, Zero};
use kludgine::app::{Window, WindowBehavior};
use kludgine::figures::units::{Lp, Px};
use kludgine::figures::{Angle, Point, Rect, Roots, Size};
use kludgine::shapes::{PathBuilder, Shape};
use kludgine::{Color, DrawableExt, PreparedGraphic};

fn main() -> Result<(), EventLoopError> {
    Test::run()
}

const BLUE_TRIANGLE_SIZE: Px = Px::new(96);
const RED_SQUARE_SIZE: Lp = Lp::inches(1);

struct Test {
    dips_square: PreparedGraphic<Lp>,
    pixels_triangle: PreparedGraphic<Px>,
    angle: Angle,
}

impl WindowBehavior for Test {
    type Context = ();

    fn initialize(
        _window: Window<'_>,
        graphics: &mut kludgine::Graphics<'_>,
        _context: Self::Context,
    ) -> Self {
        let dips_square = Shape::filled_rect(
            Rect::new(
                Point::new(-RED_SQUARE_SIZE / 2, -RED_SQUARE_SIZE / 2),
                Size::new(RED_SQUARE_SIZE, RED_SQUARE_SIZE),
            ),
            Color::RED,
        )
        .prepare(graphics);
        let height = (BLUE_TRIANGLE_SIZE.pow(2) - (BLUE_TRIANGLE_SIZE / 2).pow(2)).sqrt();
        let pixels_triangle = PathBuilder::new(Point::new(-BLUE_TRIANGLE_SIZE / 2, -height / 2))
            .line_to(Point::new(Px::ZERO, height / 2))
            .line_to(Point::new(BLUE_TRIANGLE_SIZE / 2, -height / 2))
            .close()
            .fill(Color::BLUE)
            .prepare(graphics);
        Self {
            dips_square,
            pixels_triangle,
            angle: Angle::degrees(0),
        }
    }

    fn render<'pass>(
        &'pass mut self,
        mut window: Window<'_>,
        graphics: &mut kludgine::RenderingGraphics<'_, 'pass>,
    ) {
        window.redraw_in(Duration::from_millis(16));
        self.angle += Angle::degrees(180) * window.elapsed();
        self.dips_square
            .translate_by(Point::new(RED_SQUARE_SIZE / 2, RED_SQUARE_SIZE / 2))
            .rotate_by(self.angle)
            .render(graphics);
        self.pixels_triangle
            .translate_by(Point::new(BLUE_TRIANGLE_SIZE / 2, BLUE_TRIANGLE_SIZE / 2))
            .rotate_by(self.angle)
            .render(graphics);
    }
}
