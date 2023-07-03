use appit::RunningWindow;
use cosmic_text::{Attrs, AttrsList, Buffer, Edit, Editor, Metrics};
use kludgine::app::WindowBehavior;
use kludgine::text::PreparedText;
use kludgine::Color;

fn main() {
    Test::run();
}

struct Test {
    text: Editor,
    prepared: PreparedText,
}

impl WindowBehavior for Test {
    type Context = ();

    fn initialize(
        window: &mut RunningWindow,
        graphics: &mut kludgine::Graphics<'_>,
        _context: Self::Context,
    ) -> Self {
        let mut text = Buffer::new(
            graphics.font_system(),
            Metrics::new(24.0, 32.0).scale(window.scale() as f32),
        );
        text.set_size(
            graphics.font_system(),
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        );
        let mut text = Editor::new(text);
        text.insert_string("Hello, ", None);
        text.insert_string(
            "World! 🦀",
            Some(AttrsList::new(
                Attrs::new().color(cosmic_text::Color(0x808080FF)),
            )),
        );

        text.shape_as_needed(graphics.font_system());
        let prepared = graphics.prepare_text(text.buffer(), Color::WHITE);
        Self { text, prepared }
    }

    fn prepare(&mut self, window: &mut RunningWindow, graphics: &mut kludgine::Graphics<'_>) {
        self.text.buffer_mut().set_size(
            graphics.font_system(),
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        );
        self.text.shape_as_needed(graphics.font_system());
        self.prepared = graphics.prepare_text(self.text.buffer(), Color::WHITE);
    }

    fn render<'pass>(
        &'pass mut self,
        _window: &mut RunningWindow,
        graphics: &mut kludgine::RenderingGraphics<'_, 'pass>,
    ) -> bool {
        self.prepared.render(graphics);
        true
    }
}
