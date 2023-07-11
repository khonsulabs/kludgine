use figures::{Rect, Size};
use kludgine::figures::units::Lp;
use kludgine::figures::Point;
use kludgine::shapes::Shape;
use kludgine::text::TextOrigin;
use kludgine::Color;

fn main() {
    kludgine::app::run(move |mut renderer, _window| {
        renderer.set_font_size(Lp::points(72));
        let line_height = Lp::points(72);
        renderer.set_line_height(line_height);

        let inset = Point::new(Lp::cm(1), Lp::cm(1));

        let metrics = renderer.measure_text::<Lp>("Kludgine");
        renderer.draw_shape(
            &Shape::filled_rect(
                Rect::new(
                    Point::new(metrics.left, line_height - metrics.ascent),
                    Size::new(metrics.width - metrics.left, metrics.ascent),
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
                    Point::new(metrics.left, line_height),
                    Size::new(metrics.width - metrics.left, -metrics.descent),
                ),
                Color::new(0, 0, 40, 255),
            ),
            inset,
            None,
            None,
        );

        renderer.draw_text("Kludgine", TextOrigin::TopLeft, inset, None, None);

        true
    })
}
