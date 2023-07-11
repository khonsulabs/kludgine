use std::time::Duration;

use kludgine::app::{Window, WindowBehavior};
use kludgine::figures::units::Dips;
use kludgine::figures::{Angle, Point, Rect, Size};
use kludgine::{PreparedGraphic, Texture};

fn main() {
    Test::run();
}

struct Test {
    texture: PreparedGraphic<Dips>,
    angle: Angle,
}

impl WindowBehavior for Test {
    type Context = ();

    fn initialize(
        _window: Window<'_>,
        graphics: &mut kludgine::Graphics<'_>,
        _context: Self::Context,
    ) -> Self {
        let texture = Texture::from_image(&image::open("./examples/k.png").unwrap(), graphics)
            .prepare(
                Rect::new(
                    Point::new(-Dips::inches(1) / 2, -Dips::inches(1) / 2),
                    Size::new(Dips::inches(1), Dips::inches(1)),
                ),
                graphics,
            );
        Self {
            texture,
            angle: Angle::degrees(0),
        }
    }

    fn render<'pass>(
        &'pass mut self,
        mut window: Window<'_>,
        graphics: &mut kludgine::RenderingGraphics<'_, 'pass>,
    ) -> bool {
        window.redraw_in(Duration::from_millis(16));
        self.angle += Angle::degrees(180) * window.elapsed();
        self.texture.render(
            Point::new(Dips::inches(1), Dips::inches(1)),
            None,
            Some(self.angle),
            graphics,
        );
        true
    }
}
