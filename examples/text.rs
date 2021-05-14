extern crate kludgine;
use kludgine::prelude::*;

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
    fn render(&mut self, scene: &Target<'_>) -> KludgineResult<()> {
        let scale = scene.scale_factor();
        let spans = vec![
            Span::new(
                "Wrapping ",
                Style::new()
                    .with(ForegroundColor(Color::RED.into()))
                    .with(FontSize::new(120.))
                    .to_screen_scale(scale),
            ),
            Span::new(
                "rapped ",
                Style::new()
                    .with(ForegroundColor(Color::WHITE.into()))
                    .with(FontSize::new(60.))
                    .to_screen_scale(scale),
            ),
            Span::new(
                "Words to live by",
                Style::new()
                    .with(ForegroundColor(Color::BLUE.into()))
                    .with(FontSize::new(120.))
                    .to_screen_scale(scale),
            ),
        ];

        Text::new(spans).render_at(
            scene,
            Point::new(0.0, 120.0),
            TextWrap::SingleLine {
                max_width: scene.size().width(),
                truncate: false,
                alignment: Alignment::Left,
            },
        )
    }
}
