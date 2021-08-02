use easygpu::wgpu;
use image::{GenericImageView, Rgba};
use winit::window::Theme;

use crate::{frame_renderer::FrameRenderer, prelude::*, sprite::Srgb};

#[tokio::test]
#[allow(clippy::semicolon_if_nothing_returned)] // false positive from tokio::test
async fn offscreen_render_test() {
    let (scene_sender, scene_receiver) = flume::unbounded();

    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
        })
        .await
        .expect("No wgpu adapter found");
    let renderer = easygpu::renderer::Renderer::offscreen(&adapter)
        .await
        .expect("error creating renderer");

    let mut target = Target::from(Scene::new(scene_sender, Theme::Light));
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

    let image = FrameRenderer::<Srgb>::render_one_frame(renderer, scene_receiver)
        .await
        .expect("Error rendering offscreen");

    assert_eq!(image.width(), 64);
    assert_eq!(image.height(), 64);

    assert_eq!(image.get_pixel(16, 16), Rgba([255, 0, 0, 255]));
    assert_eq!(image.get_pixel(48, 16), Rgba([0, 255, 0, 255]));
    assert_eq!(image.get_pixel(16, 48), Rgba([0, 0, 255, 255]));
    assert_eq!(image.get_pixel(48, 48), Rgba([0, 0, 0, 0]));
}
