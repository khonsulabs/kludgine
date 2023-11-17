use std::sync::OnceLock;
use std::time::Duration;

use appit::winit::error::EventLoopError;
use kludgine::figures::units::Lp;
use kludgine::figures::{Angle, IntoComponents, Point, Rect, ScreenScale, Size};
use kludgine::shapes::Shape;
use kludgine::{Color, DrawableExt, Texture};

const RED_SQUARE_SIZE: Lp = Lp::inches(1);

fn main() -> Result<(), EventLoopError> {
    // This example shows how Kludgine automatically batches drawing calls.
    // Despite the texture being drawn hundreds or thousands of times, depending
    // on the window size, the drawing calls are batched in such a way that only
    // two calls are needed to draw the entire scene.
    let mut angle = Angle::degrees(0);
    kludgine::app::run(move |mut renderer, mut window| {
        static TEXTURE: OnceLock<Texture> = OnceLock::new();
        let texture = TEXTURE.get_or_init(|| {
            Texture::from_image(&image::open("./examples/k.png").unwrap(), &renderer)
        });

        window.redraw_in(Duration::from_millis(16));
        angle += Angle::degrees(180) * window.elapsed();

        let texture_size = texture.size();
        for y in 0..(renderer.size().height / texture_size.height).0 {
            for x in 0..(renderer.size().width / texture_size.width).0 {
                renderer.draw_texture(
                    texture,
                    Rect::new(
                        Point::new(texture_size.width * x, texture_size.height * y),
                        texture_size,
                    ),
                );
            }
        }

        renderer.draw_shape(
            Shape::filled_rect(
                Rect::<Lp>::new(
                    Point::new(-RED_SQUARE_SIZE / 2, -RED_SQUARE_SIZE / 2),
                    Size::new(RED_SQUARE_SIZE, RED_SQUARE_SIZE),
                ),
                Color::RED,
            )
            .translate_by((renderer.size().into_lp(renderer.scale()) / 2).to_vec())
            .rotate_by(angle),
        );

        println!(
            "Rendering {} verticies as {} triangles in {} GPU commands. Last frame render time: {:0.02}ms",
            renderer.vertex_count(),
            renderer.triangle_count(),
            renderer.command_count(),
            window.last_frame_rendered_in().as_secs_f32() * 1000.
        );
        true
    })
}
