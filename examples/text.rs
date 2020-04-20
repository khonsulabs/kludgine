extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::<TextExample>::default().run();
}

#[derive(Default)]
struct TextExample {}

impl WindowCreator<TextExample> for TextExample {
    fn window_title() -> String {
        "Text - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for TextExample {
    async fn render(&mut self, scene: &mut Scene) -> KludgineResult<()> {
        scene.render_text_at(
            "Hello, World!",
            &bundled_fonts::ROBOTO,
            48.0,
            Point::new(0.0, 50.0),
            None,
        );

        Ok(())
    }
}
