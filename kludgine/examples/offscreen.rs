// This shows how to render offscreen using Kludgine's `Scene`. This doesn't
// quite enable full interoperability with existing `wgpu` applications. If the
// rendered texture were exposed directly, it would make Kludgine's rendering be
// buffered but still all handled on the GPU.

use kludgine::core::{
    easygpu::{renderer::Renderer, wgpu},
    flume,
    prelude::*,
};

#[tokio::main]
async fn main() {
    let (scene_sender, scene_receiver) = flume::unbounded();

    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
        })
        .await
        .expect("No wgpu adapter found");
    let renderer = Renderer::offscreen(&adapter)
        .await
        .expect("error creating renderer");

    let mut target = Target::from(Scene::new(scene_sender));
    target.scene_mut().unwrap().set_size(Size::new(64., 64.));
    target.scene_mut().unwrap().start_frame();

    Shape::circle(Point::new(16., 16.), Points::new(16.))
        .fill(Fill::new(Color::RED))
        .render_at(Point::default(), &target);

    Shape::circle(Point::new(48., 16.), Points::new(16.))
        .fill(Fill::new(Color::LIME))
        .render_at(Point::default(), &target);

    Shape::circle(Point::new(16., 48.), Points::new(16.))
        .fill(Fill::new(Color::BLUE))
        .render_at(Point::default(), &target);

    target.scene_mut().unwrap().end_frame();

    let image =
        FrameRenderer::<kludgine::core::sprite::Srgb>::render_one_frame(renderer, scene_receiver)
            .await
            .expect("Error rendering offscreen");
    let image = image.to_rgba8();
    image.save("test.png").unwrap();
}
