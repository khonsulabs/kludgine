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
    fn render(&mut self, scene: &mut SceneTarget) -> KludgineResult<()> {
        Text::new(vec![
            Span::new(
                "W",
                Style {
                    color: Some(Rgba::RED),
                    font_size: Some(120.0),
                    ..Default::default()
                },
            ),
            Span::new(
                "W",
                Style {
                    color: Some(Rgba::WHITE),
                    font_size: Some(60.0),
                    ..Default::default()
                },
            ),
            Span::new(
                "W",
                Style {
                    color: Some(Rgba::BLUE),
                    font_size: Some(120.0),
                    ..Default::default()
                },
            ),
        ])
        .render_at(
            scene,
            Point::new(0.0, 240.0),
            TextWrap::SingleLine {
                max_width: scene.size().width,
                truncate: false,
            },
        )
    }
}
