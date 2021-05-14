extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(Shapes::default());
}

#[derive(Default)]
struct Shapes;

impl WindowCreator for Shapes {
    fn window_title() -> String {
        "Shapes - Kludgine".to_owned()
    }
}

impl Window for Shapes {
    fn render(&mut self, scene: &Target<'_>) -> KludgineResult<()> {
        let center = Rect::new(Point::default(), scene.size()).center();

        Shape::polygon(vec![
            Point::new(-100., -100.),
            Point::new(0., 100.),
            Point::new(100., -100.),
        ])
        .fill(Fill::new(Color::GREEN))
        .render_at(center, scene);

        Shape::circle(Point::new(0., 0.), Points::new(25.))
            .fill(Fill::new(Color::RED))
            .render_at(center, scene);

        Ok(())
    }
}
