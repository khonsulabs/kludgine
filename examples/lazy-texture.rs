use appit::winit::error::EventLoopError;
use kludgine::figures::{Lp2D, Point};
use kludgine::Texture;

fn main() -> Result<(), EventLoopError> {
    let texture = Texture::lazy_from_image(
        image::open("./examples/assets/k.png").unwrap(),
        wgpu::FilterMode::Linear,
    );
    kludgine::app::run(move |mut renderer, _window| {
        renderer.draw_texture_at(&texture, Point::inches(1, 1), 1.);
        true
    })
}
