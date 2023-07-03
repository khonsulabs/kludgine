use std::time::Duration;

use appit::RunningWindow;
use kludgine::app::WindowBehavior;
use kludgine::math::{Dips, Point, Rect, Size};
use kludgine::PreparedGraphic;
use kludgine::Texture;

fn main() {
    Test::run();
}

struct Test {
    texture: PreparedGraphic<Dips>,
    angle: f32,
}

impl WindowBehavior for Test {
    type Context = ();

    fn initialize(
        _window: &mut RunningWindow,
        graphics: &mut kludgine::Graphics<'_>,
        _context: Self::Context,
    ) -> Self {
        let texture = Texture::from_image(&image::open("./examples/k.png").unwrap(), graphics)
            .prepare(
                Rect::new(
                    Point::new(-Dips::INCH / 2, -Dips::INCH / 2),
                    Size::new(Dips::INCH, Dips::INCH),
                ),
                graphics,
            );
        Self { texture, angle: 0. }
    }

    fn render<'pass>(
        &'pass mut self,
        window: &mut RunningWindow,
        graphics: &mut kludgine::RenderingGraphics<'_, 'pass>,
    ) -> bool {
        window.redraw_in(Duration::from_millis(16));
        self.angle += std::f32::consts::PI / 36.;
        self.texture.render(
            Point::new(Dips::INCH, Dips::INCH),
            None,
            Some(self.angle),
            graphics,
        );
        true
    }
}
