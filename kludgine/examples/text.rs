use kludgine::{core::text::bundled_fonts::ROBOTO, prelude::*};

fn main() {
    SingleWindowApplication::run(TextExample {});
}

struct TextExample {}

impl WindowCreator for TextExample {
    fn window_title() -> String {
        "Text - Kludgine".to_owned()
    }
}

impl Window for TextExample {
    fn render(
        &mut self,
        scene: &Target,
        _status: &mut RedrawStatus,
        _window: WindowHandle,
    ) -> kludgine::Result<()> {
        Text::prepare(
            "Hello, World!",
            &ROBOTO,
            Figure::new(64.),
            Color::BISQUE,
            scene,
        )
        .render_baseline_at(scene, Point::<f32, Scaled>::new(64., 64.))?;
        Ok(())
    }
}
