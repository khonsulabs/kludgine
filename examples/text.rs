use kludgine::{prelude::*, text::bundled_fonts::ROBOTO};

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
    fn render(&mut self, scene: &Target) -> KludgineResult<()> {
        Text::prepare(
            "Hello, World!",
            &ROBOTO,
            Points::new(64.),
            Color::BISQUE,
            scene,
        )
        .render_baseline_at(scene, Point::new(64., 64.))
    }
}
