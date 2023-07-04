use std::time::Duration;

use kludgine::app::{Window, WindowBehavior};
use kludgine::figures::{Dips, Pixels, Point, Rect, Size};
use kludgine::shapes::{PathBuilder, Shape};
use kludgine::{Color, PreparedGraphic};

fn main() {
    Test::run();
}

const BLUE_TRIANGLE_SIZE: Pixels = Pixels(32);
const RED_SQUARE_SIZE: Dips = Dips::INCH;

struct Test {
    dips_square: PreparedGraphic<Dips>,
    pixels_triangle: PreparedGraphic<Pixels>,
    angle: f32,
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
        let pixels_triangle =
            PathBuilder::new(Point::new(-BLUE_TRIANGLE_SIZE, -BLUE_TRIANGLE_SIZE))
                .line_to(Point::new(0, BLUE_TRIANGLE_SIZE))
                .line_to(Point::new(BLUE_TRIANGLE_SIZE, -BLUE_TRIANGLE_SIZE))
                .close()
                .fill(Color::BLUE)
                .prepare(graphics);
        Self {
            dips_square,
            pixels_triangle,
            angle: 0.,
        }
    }

    fn render<'pass>(
        &'pass mut self,
        mut window: Window<'_>,
        graphics: &mut kludgine::RenderingGraphics<'_, 'pass>,
    ) -> bool {
        window.redraw_in(Duration::from_millis(16));
        self.angle += std::f32::consts::PI / 36.;
        self.dips_square.render(
            Point::new(RED_SQUARE_SIZE / 2, RED_SQUARE_SIZE / 2),
            None,
            Some(self.angle),
            graphics,
        );
        self.pixels_triangle.render(
            Point::new(BLUE_TRIANGLE_SIZE, BLUE_TRIANGLE_SIZE),
            None,
            Some(self.angle),
            graphics,
        );
        true
    }
}
