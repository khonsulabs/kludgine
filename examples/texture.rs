use std::time::Duration;

use appit::winit::error::EventLoopError;
use kludgine::app::{Window, WindowBehavior};
use kludgine::figures::units::Lp;
use kludgine::figures::{Angle, Lp2D, Point, Rect, Size};
use kludgine::{PreparedGraphic, Texture};

fn main() -> Result<(), EventLoopError> {
    Test::run()
}

struct Test {
    texture: PreparedGraphic<Lp>,
    angle: Angle,
}

impl WindowBehavior for Test {
    type Context = ();

    fn initialize(
        _window: Window<'_>,
        graphics: &mut kludgine::Graphics<'_>,
        _context: Self::Context,
    ) -> Self {
        let texture = Texture::from_image(
            image::open("./examples/assets/k.png").unwrap(),
            wgpu::FilterMode::Linear,
            graphics,
        )
        .prepare(
            Rect::new(-Point::inches(1, 1) / 2, Size::inches(1, 1)),
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
        self.texture
            .render(Point::inches(1, 1), None, Some(self.angle), graphics);
        true
    }
}
