extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(TextExample {});
}

struct TextExample {}

impl WindowCreator<TextExample> for TextExample {
    fn window_title() -> String {
        "Text - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for TextExample {
    async fn render<'a>(&mut self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        Text::new(vec![
            Span::new(
                "W",
                Style {
                    color: Some(Color::RED),
                    font_size: Some(120.0),
                    ..Default::default()
                }
                .effective_style(scene),
            ),
            Span::new(
                "W",
                Style {
                    color: Some(Color::WHITE),
                    font_size: Some(60.0),
                    ..Default::default()
                }
                .effective_style(scene),
            ),
            Span::new(
                "W",
                Style {
                    color: Some(Color::BLUE),
                    font_size: Some(120.0),
                    ..Default::default()
                }
                .effective_style(scene),
            ),
        ])
        .render_at(
            scene,
            Point::new(0.0, 120.0),
            TextWrap::SingleLine {
                max_width: scene.size().width,
                truncate: false,
            },
        )
        .await
    }
}
