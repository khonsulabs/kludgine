use std::time::Duration;

use appit::winit::error::EventLoopError;
use figures::{Angle, FromComponents, Point};
use kludgine::app::{Window, WindowBehavior};
use kludgine::cosmic_text::{Attrs, AttrsList, Buffer, Edit, Editor, Metrics};
use kludgine::figures::{FloatConversion, ScreenScale};
use kludgine::text::{PreparedText, TextOrigin};
use kludgine::{Color, DrawableExt};

fn main() -> Result<(), EventLoopError> {
    Test::run()
}

struct Test {
    text: Buffer,
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
            Some(size.width.into_float()),
            Some(size.height.into_float()),
        );
        let mut editor = Editor::new(&mut text);
        editor.insert_string("Hello, ", None);
        editor.insert_string(
            "World! 🦀",
            Some(AttrsList::new(
                Attrs::new().color(cosmic_text::Color(0x808080FF)),
            )),
        );
        // A right-to-left text string, borrowed from
        // <https://en.wikipedia.org/wiki/Right-to-left_mark#Example_of_use_in_HTML>.
        // The exclamation mark should be rendered to the left of the Hebrew
        // characters.
        editor.insert_string("\nI enjoyed staying -- באמת!‏ -- at his house.", None);

        editor.shape_as_needed(graphics.font_system(), true);
        let prepared = graphics.prepare_text(&text, Color::WHITE, TextOrigin::Center);
        Self {
            text,
            prepared,
            angle: Angle::degrees(0),
        }
    }

    fn prepare(&mut self, _window: Window<'_>, graphics: &mut kludgine::Graphics<'_>) {
        let scale = graphics.scale();
        let size = graphics.size();
        self.text.set_size(
            graphics.font_system(),
            Some(size.width.into_float()),
            Some(size.height.into_float()),
        );
        self.text.set_metrics(
            graphics.font_system(),
            Metrics::new(24.0, 24.0).scale(scale.into_f32()),
        );
        let mut editor = Editor::new(&mut self.text);
        editor.shape_as_needed(graphics.font_system(), true);
        self.prepared = graphics.prepare_text(&self.text, Color::WHITE, TextOrigin::Center);
    }

    fn render<'pass>(
        &'pass mut self,
        mut window: Window<'_>,
        graphics: &mut kludgine::RenderingGraphics<'_, 'pass>,
    ) {
        window.redraw_in(Duration::from_millis(16));
        self.angle += Angle::degrees(180) * window.elapsed() / 5;
        self.prepared
            .translate_by(Point::from_vec(graphics.size().into_px(graphics.scale())) / 2)
            .rotate_by(self.angle)
            .render(graphics);
    }
}
