use std::time::Duration;

use appit::winit::error::EventLoopError;
use kludgine::app::{Window, WindowBehavior};
use kludgine::figures::units::{Lp, Px};
use kludgine::figures::{Angle, Lp2D, Point, Rect, ScreenScale, Size};
use kludgine::{Color, PreparedGraphic, TextureCollection};

fn main() -> Result<(), EventLoopError> {
    Test::run()
}

struct Test {
    atlas: PreparedGraphic<Px>,
    k: PreparedGraphic<Lp>,
    ferris: PreparedGraphic<Lp>,
    angle: Angle,
}

impl WindowBehavior for Test {
    type Context = ();

    fn initialize(
        _window: Window<'_>,
        graphics: &mut kludgine::Graphics<'_>,
        _context: Self::Context,
    ) -> Self {
        let mut textures = TextureCollection::new(
            Size::new(1024, 1024).cast(),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            graphics,
        );
        let k = textures.push_image(&image::open("./examples/assets/k.png").unwrap(), graphics);
        let k = k.prepare(
            Rect::new(-Point::inches(1, 1) / 2, Size::inches(1, 1)),
            graphics,
        );
        let ferris = textures.push_image(
            &image::open("./examples/assets/ferris-happy.png").unwrap(),
            graphics,
        );
        let ferris = ferris.prepare(
            Rect::new(-Point::inches(1, 0.75) / 2, Size::inches(1, 0.75)),
            graphics,
        );
        let atlas = textures.prepare_entire_colection(Size::new(256, 256).cast().into(), graphics);

        Self {
            atlas,
            k,
            ferris,
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
        self.k
            .render(Point::inches(1, 1), None, Some(self.angle), graphics);
        self.ferris
            .render(Point::inches(2, 1), None, Some(-self.angle), graphics);
        self.atlas.render(
            Point::new(
                0,
                graphics.size().height.into_px(graphics.scale()).get() - 256,
            )
            .cast(),
            None,
            None,
            graphics,
        );
        true
    }

    fn clear_color(&self) -> Option<Color> {
        Some(Color::new(10, 0, 0, 255))
    }
}
