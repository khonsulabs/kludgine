use std::time::Duration;

use figures::traits::FromComponents;
use figures::{Angle, Point};
use kludgine::app::{Window, WindowBehavior};
use kludgine::cosmic_text::{Attrs, AttrsList, Buffer, Edit, Editor, Metrics};
use kludgine::figures::traits::{FloatConversion, ScreenScale};
use kludgine::text::{PreparedText, TextOrigin};
use kludgine::Color;

fn main() {
    Test::run();
}

struct Test {
    text: Editor,
    prepared: PreparedText,
    angle: Angle,
}

impl WindowBehavior for Test {
    type Context = ();

    fn initialize(
        _window: Window<'_>,
        graphics: &mut kludgine::Graphics<'_>,
        _context: Self::Context,
    ) -> Self {
        let scale = graphics.scale();
        let size = graphics.size();
        let mut text = Buffer::new(
            graphics.font_system(),
            Metrics::new(24.0, 24.0).scale(scale.into_f32()),
        );
        text.set_size(
            graphics.font_system(),
            size.width.into_float(),
            size.height.into_float(),
        );
        let mut text = Editor::new(text);
        text.insert_string("Hello, ", None);
        text.insert_string(
            "World! 🦀",
            Some(AttrsList::new(
                Attrs::new().color(cosmic_text::Color(0x808080FF)),
            )),
        );
        // A right-to-left text string, borrowed from
        // <https://en.wikipedia.org/wiki/Right-to-left_mark#Example_of_use_in_HTML>.
        // The exclamation mark should be rendered to the left of the Hebrew
        // characters.
        text.insert_string("\nI enjoyed staying -- באמת!‏ -- at his house.", None);

        text.shape_as_needed(graphics.font_system());
        let prepared = graphics.prepare_text(text.buffer(), Color::WHITE, TextOrigin::Center);
        Self {
            text,
            prepared,
            angle: Angle::degrees(0),
        }
    }

    fn prepare(&mut self, _window: Window<'_>, graphics: &mut kludgine::Graphics<'_>) {
        let scale = graphics.scale();
        let size = graphics.size();
        self.text.buffer_mut().set_size(
            graphics.font_system(),
            size.width.into_float(),
            size.height.into_float(),
        );
        self.text.buffer_mut().set_metrics(
            graphics.font_system(),
            Metrics::new(24.0, 24.0).scale(scale.into_f32()),
        );
        self.text.shape_as_needed(graphics.font_system());
        self.prepared = graphics.prepare_text(self.text.buffer(), Color::WHITE, TextOrigin::Center);
    }

    fn render<'pass>(
        &'pass mut self,
        mut window: Window<'_>,
        graphics: &mut kludgine::RenderingGraphics<'_, 'pass>,
    ) -> bool {
        window.redraw_in(Duration::from_millis(16));
        self.angle += Angle::degrees(180) * window.elapsed() / 5;
        self.prepared.render(
            Point::from_vec(graphics.size().into_px(graphics.scale())) / 2,
            None,
            Some(self.angle),
            graphics,
        );
        true
    }
}
