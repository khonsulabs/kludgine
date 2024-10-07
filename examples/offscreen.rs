use appit::winit::error::EventLoopError;
use kludgine::app::{Window, WindowBehavior};
use kludgine::figures::units::Px;
use kludgine::figures::{Angle, Point, Px2D, Rect, Size};
use kludgine::shapes::Shape;
use kludgine::{
    Color, DrawableExt, Graphics, Kludgine, PreparedGraphic, RenderingGraphics, Texture,
};

fn main() -> Result<(), EventLoopError> {
    Test::run()
}

struct Test {
    prepared: PreparedGraphic<Px>,
}

impl WindowBehavior for Test {
    type Context = ();

    fn initialize(
        _window: Window<'_>,
        graphics: &mut Graphics<'_>,
        _context: Self::Context,
    ) -> Self {
        let prerendered = Texture::new(
            graphics,
            Size::new(512, 512).cast(),
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            wgpu::FilterMode::Linear,
        );
        let mut kludgine = Kludgine::new(
            graphics.device(),
            graphics.queue(),
            prerendered.format(),
            wgpu::MultisampleState::default(),
            Size::new(512, 512).cast(),
            1.0,
        );
        let mut frame = kludgine.next_frame();
        // Prepare the graphics.
        let preparing = frame.prepare(graphics.device(), graphics.queue());
        let square_size = Px::from((512f32.powf(2.) / 2.).sqrt());
        let outer_square = Shape::filled_rect(
            Rect::<Px>::new(
                Point::new(-square_size / 2, -square_size / 2),
                Size::new(square_size, square_size),
            ),
            Color::RED,
        )
        .prepare(&preparing);

        // Render the texture
        let mut rendering = frame.render_into(
            &prerendered,
            wgpu::LoadOp::Clear(Color::WHITE),
            graphics.device(),
            graphics.queue(),
        );
        outer_square
            .translate_by(Point::px(256, 256))
            .rotate_by(Angle::degrees(45))
            .render(&mut rendering);
        drop(rendering);

        frame.submit(graphics.queue());

        Self {
            prepared: prerendered.prepare(Size::new(400, 400).cast().into(), graphics),
        }
    }

    fn render<'pass>(
        &'pass mut self,
        _window: Window<'_>,
        graphics: &mut RenderingGraphics<'_, 'pass>,
    ) {
        self.prepared.render(graphics);
    }
}
