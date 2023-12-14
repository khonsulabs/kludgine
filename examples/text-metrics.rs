use appit::winit::error::EventLoopError;
use figures::{Lp2D, Rect, Size};
use kludgine::figures::units::Lp;
use kludgine::figures::Point;
use kludgine::shapes::Shape;
use kludgine::text::TextOrigin;
use kludgine::{Color, DrawableExt};

fn main() -> Result<(), EventLoopError> {
    kludgine::app::run(move |mut renderer, _window| {
        renderer.set_font_size(Lp::points(72));
        let line_height = Lp::points(72);
        renderer.set_line_height(line_height);

        let inset = Point::cm(1, 1);

        let measured = renderer.measure_text::<Lp>("Kludgine");
        renderer.draw_shape(
            Shape::filled_rect(
                Rect::new(
                    Point::new(measured.left, line_height - measured.ascent),
                    Size::new(measured.size.width - measured.left, measured.ascent),
                ),
                Color::new(0, 255, 0, 128),
            )
            .translate_by(inset),
        );
        renderer.draw_shape(
            Shape::filled_rect(
                Rect::new(
                    Point::new(measured.left, line_height),
                    Size::new(measured.size.width - measured.left, -measured.descent),
                ),
                Color::new(0, 0, 255, 128),
            )
            .translate_by(inset),
        );

        renderer.draw_measured_text(measured.translate_by(inset), TextOrigin::TopLeft);

        true
    })
}
