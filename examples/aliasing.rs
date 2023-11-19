use appit::winit::error::EventLoopError;
use figures::{Angle, Lp2D, Rect, Size};
use kludgine::figures::units::Lp;
use kludgine::figures::{Point, Px2D};
use kludgine::shapes::{PathBuilder, Shape, StrokeOptions};
use kludgine::{Color, DrawableExt};

fn main() -> Result<(), EventLoopError> {
    kludgine::app::run(move |mut renderer, _| {
        renderer.draw_shape(
            PathBuilder::new(Point::cm(1, 1))
                .line_to(Point::cm(30, 1))
                .build()
                .stroke(StrokeOptions::mm_wide(2).colored(Color::RED))
                .rotate_by(Angle::degrees(30)),
        );
        renderer.draw_shape(
            Shape::filled_rect(
                Rect::new(Point::cm(0, 0), Size::squared(Lp::cm(5))),
                Color::RED,
            )
            .rotate_by(Angle::degrees(55))
            .translate_by(Point::cm(5, 5)),
        );
        renderer.draw_shape(
            Shape::filled_rect(
                Rect::new(Point::cm(0, 0), Size::squared(Lp::mm(49))),
                Color::RED,
            )
            .rotate_by(Angle::degrees(55))
            .translate_by(Point::cm(10, 5)),
        );
        renderer.draw_shape(
            Shape::stroked_rect(
                Rect::new(Point::cm(0, 0), Size::squared(Lp::mm(49))),
                StrokeOptions::mm_wide(2).colored(Color::RED),
            )
            .rotate_by(Angle::degrees(55))
            .translate_by(Point::cm(10, 5)),
        );
        renderer.draw_shape(
            PathBuilder::new(Point::cm(1, 1))
                .line_to(Point::cm(30, 1))
                .build()
                .stroke(Color::LIGHTBLUE)
                .rotate_by(Angle::degrees(69)),
        );
        renderer.draw_shape(
            PathBuilder::default()
                .line_to(Point::px(300, 0))
                .build()
                .stroke(StrokeOptions::px_wide(1).colored(Color::LIGHTBLUE))
                .rotate_by(Angle::degrees(69))
                .translate_by(Point::px(200, 200)),
        );
        renderer.draw_shape(
            PathBuilder::default()
                .line_to(Point::px(300, 0))
                .build()
                .stroke(StrokeOptions::px_wide(1).colored(Color::LIGHTBLUE))
                .translate_by(Point::px(200, 200)),
        );
        true
    })
}
