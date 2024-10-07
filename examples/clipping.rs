use std::time::Duration;

use appit::winit::error::EventLoopError;
use kludgine::app::{Window, WindowBehavior};
use kludgine::figures::units::Px;
use kludgine::figures::{Angle, Point, Px2D, Rect, ScreenScale, Size};
use kludgine::shapes::Shape;
use kludgine::{Color, DrawableExt, PreparedGraphic};

fn main() -> Result<(), EventLoopError> {
    Test::run()
}

struct Test {
    red_square: PreparedGraphic<Px>,
    blue_square: PreparedGraphic<Px>,
    angle: Angle,
}

impl WindowBehavior for Test {
    type Context = ();

    fn initialize(
        _window: Window<'_>,
        graphics: &mut kludgine::Graphics<'_>,
        _context: Self::Context,
    ) -> Self {
        let outer_square = Shape::filled_rect(
            Rect::new(Point::px(-200, -200), Size::px(400, 400)),
            Color::RED,
        )
        .prepare(graphics);
        let inner_square = Shape::filled_rect(
            Rect::new(Point::px(-50, -50), Size::px(100, 100)),
            Color::BLUE,
        )
        .prepare(graphics);

        Self {
            red_square: outer_square,
            blue_square: inner_square,
            angle: Angle::degrees(0),
        }
    }

    fn render<'pass>(
        &'pass mut self,
        mut window: Window<'_>,
        graphics: &mut kludgine::RenderingGraphics<'_, 'pass>,
    ) {
        window.redraw_in(Duration::from_millis(16));
        self.angle += window.elapsed().as_secs_f32() / 5.;

        let mut clipped = graphics.clipped_to(Rect::new(
            Point::from(graphics.size()) / 4,
            graphics.size() / 2,
        ));
        // `clipped` now acts as if 0,0 is at `clip_origin`. The borrow checker
        // prevents using `graphics` until `clipped` is dropped.
        self.red_square
            .translate_by(Point::from(clipped.size()).into_px(clipped.scale()) / 2)
            .rotate_by(self.angle)
            .render(&mut clipped);
        self.blue_square.rotate_by(-self.angle).render(&mut clipped);
        drop(clipped);

        // Now `graphics` can be used without clipping again.
        self.blue_square
            .translate_by(Point::from(graphics.size().into_px(graphics.scale())) / 4 * 3)
            .rotate_by(self.angle)
            .render(graphics);
    }
}
