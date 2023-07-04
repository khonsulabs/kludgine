use std::time::Duration;

use figures::Point;
use kludgine::app::{Window, WindowBehavior};
use kludgine::cosmic_text::{Attrs, AttrsList, Buffer, Edit, Editor, Metrics};
use kludgine::figures::traits::FloatConversion;
use kludgine::figures::units::Px;
use kludgine::text::{PreparedText, TextOrigin};
use kludgine::Color;

fn main() {
    Test::run();
}

struct Test {
    text: Editor,
    prepared: PreparedText,
    angle: f32,
}

impl WindowBehavior for Test {
    type Context = ();

    fn initialize(
        window: Window<'_>,
        graphics: &mut kludgine::Graphics<'_>,
        _context: Self::Context,
    ) -> Self {
        let mut text = Buffer::new(
            graphics.font_system(),
            Metrics::new(24.0, 32.0).scale(window.scale()),
        );
        text.set_size(
            graphics.font_system(),
            window.inner_size().width.into_float(),
            window.inner_size().height.into_float(),
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
            angle: 0.,
        }
    }

    fn prepare(&mut self, window: Window<'_>, graphics: &mut kludgine::Graphics<'_>) {
        self.text.buffer_mut().set_size(
            graphics.font_system(),
            window.inner_size().width.into_float(),
            window.inner_size().height.into_float(),
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
        self.angle += std::f32::consts::PI * window.elapsed().as_secs_f32() / 5.;
        self.prepared.render(
            Point::from(window.inner_size()).try_cast::<Px>().unwrap() / 2,
            None,
            Some(self.angle),
            graphics,
        );
        true
    }
}
