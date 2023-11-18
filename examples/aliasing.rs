use appit::winit::error::EventLoopError;
use figures::units::Px;
use figures::{Angle, Rect, Size};
use kludgine::figures::units::Lp;
use kludgine::figures::Point;
use kludgine::shapes::{PathBuilder, Shape, StrokeOptions};
use kludgine::{Color, DrawableExt};

fn main() -> Result<(), EventLoopError> {
    kludgine::app::run(move |mut renderer, _| {
        renderer.draw_shape(
            PathBuilder::<Lp, false>::new(Point::new(Lp::cm(1), Lp::cm(1)))
                .line_to(Point::new(Lp::cm(30), Lp::cm(1)))
                .build()
                .stroke(Color::RED, StrokeOptions::lp_wide(Lp::mm(2)))
                .rotate_by(Angle::degrees(30)),
        );
        renderer.draw_shape(
            Shape::filled_rect(
                Rect::<Lp>::new(Point::new(Lp::cm(0), Lp::cm(0)), Size::squared(Lp::cm(5))),
                Color::RED,
            )
            .rotate_by(Angle::degrees(55))
            .translate_by(Point::new(Lp::cm(5), Lp::cm(5))),
        );
        renderer.draw_shape(
            Shape::filled_rect(
                Rect::<Lp>::new(Point::new(Lp::cm(0), Lp::cm(0)), Size::squared(Lp::mm(49))),
                Color::RED,
            )
            .rotate_by(Angle::degrees(55))
            .translate_by(Point::new(Lp::cm(10), Lp::cm(5))),
        );
        renderer.draw_shape(
            Shape::stroked_rect(
                Rect::<Lp>::new(Point::new(Lp::cm(0), Lp::cm(0)), Size::squared(Lp::mm(49))),
                Color::RED,
                StrokeOptions::lp_wide(Lp::mm(2)),
            )
            .rotate_by(Angle::degrees(55))
            .translate_by(Point::new(Lp::cm(10), Lp::cm(5))),
        );
        renderer.draw_shape(
            PathBuilder::<Lp, false>::new(Point::new(Lp::cm(1), Lp::cm(1)))
                .line_to(Point::new(Lp::cm(30), Lp::cm(1)))
                .build()
                .stroke(Color::LIGHTBLUE, StrokeOptions::default())
                .rotate_by(Angle::degrees(69)),
        );
        renderer.draw_shape(
            PathBuilder::<Px, false>::new(Point::new(0.into(), 0.into()))
                .line_to(Point::new(Px(300), Px(0)))
                .build()
                .stroke(Color::LIGHTBLUE, StrokeOptions::px_wide(1))
                .rotate_by(Angle::degrees(69))
                .translate_by(Point::new(Px(200), Px(200))),
        );
        // renderer.draw_shape(
        //     &Shape::stroked_rect(
        //         Rect::<Lp>::new(
        //             Point::new(-RED_SQUARE_SIZE / 2, -RED_SQUARE_SIZE / 2),
        //             Size::new(RED_SQUARE_SIZE, RED_SQUARE_SIZE),
        //         ),
        //         Color::RED,
        //         StrokeOptions::default(),
        //     ),
        //     shape_center,
        //     Some(Angle::degrees(30)),
        //     None,
        // );
        // renderer.draw_shape(
        //     &Shape::stroked_rect(
        //         Rect::<Lp>::new(
        //             Point::new(-RED_SQUARE_SIZE / 2, -RED_SQUARE_SIZE / 2),
        //             Size::new(RED_SQUARE_SIZE, RED_SQUARE_SIZE),
        //         ),
        //         Color::RED,
        //         StrokeOptions::default(),
        //     ),
        //     shape_center,
        //     None,
        //     None,
        // );
        // renderer.draw_text(
        //     Text::new("Hello, World!", Color::WHITE).origin(TextOrigin::Center),
        //     shape_center,
        //     Some(Angle::degrees(30)),
        //     None,
        // );
        true
    })
}
