// This shows how to render offscreen using Kludgine's `Scene`. This doesn't
// quite enable full interoperability with existing `wgpu` applications. If the
// rendered texture were exposed directly, it would make Kludgine's rendering be
// buffered but still all handled on the GPU.

use kludgine::core::{
    easygpu::{renderer::Renderer, wgpu},
    flume,
    prelude::*,
};
use kludgine_core::winit::window::Theme;

#[tokio::main]
async fn main() {
    let (scene_sender, scene_receiver) = flume::unbounded();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .expect("No wgpu adapter found");
    let renderer = Renderer::offscreen(&adapter, 4)
        .await
        .expect("error creating renderer");

    let mut target = Target::from(Scene::new(scene_sender, Theme::Light));
    target.scene_mut().unwrap().set_size(Size::new(64, 64));
    target.scene_mut().unwrap().start_frame();

    Shape::<Scaled>::circle(Point::new(16., 16.), Figure::new(16.))
        .fill(Fill::new(Color::RED))
        .render(&target);

    Shape::<Scaled>::circle(Point::new(48., 16.), Figure::new(16.))
        .fill(Fill::new(Color::LIME))
        .render(&target);

    Shape::<Scaled>::circle(Point::new(16., 48.), Figure::new(16.))
        .fill(Fill::new(Color::BLUE))
        .render(&target);

    target.scene_mut().unwrap().end_frame();

    let image =
        FrameRenderer::<kludgine::core::sprite::Srgb>::render_one_frame(renderer, scene_receiver)
            .expect("Error rendering offscreen");
    let image = image.to_rgba8();
    image.save("test.png").unwrap();
}
