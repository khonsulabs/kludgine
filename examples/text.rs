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

impl Window for TextExample {}

impl StandaloneComponent for TextExample {}

#[async_trait]
impl Component for TextExample {
    async fn render(&self, context: &mut StyledContext, _layout: &Layout) -> KludgineResult<()> {
        let mut spans = Vec::new();
        spans.push(Span::new(
            "Wrapping ",
            Style::new()
                .with(ForegroundColor(Color::RED))
                .with(FontSize::new(120.))
                .effective_style(context.scene())
                .await,
        ));
        spans.push(Span::new(
            "rapped ",
            Style::new()
                .with(ForegroundColor(Color::WHITE))
                .with(FontSize::new(60.))
                .effective_style(context.scene())
                .await,
        ));
        spans.push(Span::new(
            "Words to live by",
            Style::new()
                .with(ForegroundColor(Color::BLUE))
                .with(FontSize::new(120.))
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
