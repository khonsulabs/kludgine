use std::time::Duration;

use kludgine::app::{Window, WindowBehavior};
use kludgine::figures::units::{Dip, Px};
use kludgine::figures::{Point, Rect, Size};
use kludgine::{Color, PreparedGraphic, TextureCollection};

fn main() {
    Test::run();
}

struct Test {
    atlas: PreparedGraphic<Px>,
    k: PreparedGraphic<Dip>,
    ferris: PreparedGraphic<Dip>,
    angle: f32,
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
                Point::new(-Dip::INCH / 2, -Dip::INCH / 2),
                Size::new(Dip::INCH, Dip::INCH),
            ),
            graphics,
        );
        let ferris = textures.push_image(
            &image::open("./examples/ferris-happy.png").unwrap(),
            graphics,
        );
        let ferris = ferris.prepare(
            Rect::new(
                Point::new(-Dip::INCH / 2, -Dip::INCH / 2),
                Size::new(Dip::INCH, Dip::INCH / 1.5),
            ),
            graphics,
        );
        let atlas = textures.prepare_entire_colection(Size::new(256, 256).into(), graphics);

        Self {
            atlas,
            k,
            ferris,
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
        self.k.render(
            Point::new(Dip::INCH, Dip::INCH),
            None,
            Some(self.angle),
            graphics,
        );
        self.ferris.render(
            Point::new(Dip::INCH * 2, Dip::INCH),
            None,
            Some(-self.angle),
            graphics,
        );
        self.atlas.render(
            Point::new(0, window.inner_size().height.0 as i32 - 256),
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
