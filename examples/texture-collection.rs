use std::time::Duration;

use kludgine::app::{Window, WindowBehavior};
use kludgine::figures::units::{Lp, Px};
use kludgine::figures::{Angle, Point, Rect, ScreenScale, Size};
use kludgine::{Color, PreparedGraphic, TextureCollection};

fn main() {
    Test::run();
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
            Size::new(1024, 1024),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            graphics,
        );
        let k = textures.push_image(&image::open("./examples/k.png").unwrap(), graphics);
        let k = k.prepare(
            Rect::new(
                Point::new(-Lp::inches(1) / 2, -Lp::inches(1) / 2),
                Size::new(Lp::inches(1), Lp::inches(1)),
            ),
            graphics,
        );
        let ferris = textures.push_image(
            &image::open("./examples/ferris-happy.png").unwrap(),
            graphics,
        );
        let ferris = ferris.prepare(
            Rect::new(
                Point::new(-Lp::inches(1) / 2, -Lp::inches(1) / 2),
                Size::new(Lp::inches(1), Lp::inches(1) / 1.5),
            ),
            graphics,
        );
        let atlas = textures.prepare_entire_colection(Size::new(256, 256).into(), graphics);

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
        self.k.render(
            Point::new(Lp::inches(1), Lp::inches(1)),
            None,
            Some(self.angle),
            graphics,
        );
        self.ferris.render(
            Point::new(Lp::inches(1) * 2, Lp::inches(1)),
            None,
            Some(-self.angle),
            graphics,
        );
        self.atlas.render(
            Point::new(0, graphics.size().height.into_px(graphics.scale()).0 - 256).cast(),
            None,
            None,
            graphics,
        );
        true
    }

    fn clear_color() -> Option<Color> {
        Some(Color::new(10, 0, 0, 255))
    }
}
