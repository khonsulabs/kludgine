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

impl Window for TextExample {}

impl StandaloneComponent for TextExample {}

#[async_trait]
impl Component for TextExample {
    async fn render(&self, context: &mut StyledContext, _layout: &Layout) -> KludgineResult<()> {
        let mut spans = Vec::new();
        spans.push(Span::new(
            "Wrapping ",
            Style {
                color: Some(Color::RED),
                font_size: Some(Points::new(120.0)),
                ..Default::default()
            }
            .effective_style(context.scene())
            .await,
        ));
        spans.push(Span::new(
            "rapped ",
            Style {
                color: Some(Color::WHITE),
                font_size: Some(Points::new(60.0)),
                ..Default::default()
            }
            .effective_style(context.scene())
            .await,
        ));
        spans.push(Span::new(
            "Words to live by",
            Style {
                color: Some(Color::BLUE),
                font_size: Some(Points::new(120.0)),
                ..Default::default()
            }
            .effective_style(context.scene())
            .await,
        ));

        Text::new(spans)
            .render_at(
                context.scene(),
                Point::new(0.0, 120.0),
                TextWrap::SingleLine {
                    max_width: context.scene().size().await.width(),
                    truncate: false,
                    alignment: Alignment::Left,
                },
            )
            .await
    }
}
