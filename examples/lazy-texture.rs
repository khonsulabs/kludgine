use appit::winit::error::EventLoopError;
use kludgine::figures::{Lp2D, Point};
use kludgine::LazyTexture;

fn main() -> Result<(), EventLoopError> {
    let texture = LazyTexture::from_image(
        image::open("./examples/assets/k.png").unwrap(),
        wgpu::FilterMode::Linear,
    );
    kludgine::app::run(move |mut renderer, _window| {
        renderer.draw_texture_at(&texture, Point::inches(1, 1), 1.);
        true
    })
}
