use appit::winit::error::EventLoopError;
use figures::{Rect, Size};
use kludgine::figures::units::Lp;
use kludgine::figures::Point;
use kludgine::shapes::Shape;
use kludgine::text::TextOrigin;
use kludgine::Color;

fn main() -> Result<(), EventLoopError> {
    kludgine::app::run(move |mut renderer, _window| {
        renderer.set_font_size(Lp::points(72));
        let line_height = Lp::points(72);
        renderer.set_line_height(line_height);

        let inset = Point::new(Lp::cm(1), Lp::cm(1));

        let measured = renderer.measure_text::<Lp>("Kludgine");
        renderer.draw_shape(
            &Shape::filled_rect(
                Rect::new(
                    Point::new(measured.left, line_height - measured.ascent),
                    Size::new(measured.size.width - measured.left, measured.ascent),
                ),
                Color::new(0, 40, 0, 255),
            ),
            inset,
            None,
            None,
        );
        renderer.draw_shape(
            &Shape::filled_rect(
                Rect::new(
                    Point::new(measured.left, line_height),
                    Size::new(measured.size.width - measured.left, -measured.descent),
                ),
                Color::new(0, 0, 40, 255),
            ),
            inset,
            None,
            None,
        );

        renderer.draw_measured_text(&measured, TextOrigin::TopLeft, inset, None, None);

        true
    })
}
