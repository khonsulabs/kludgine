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

#[async_trait]
impl Window for Shapes {
    async fn render(&mut self, scene: &Target) -> KludgineResult<()> {
        let center = Rect::new(Point::default(), scene.size().await).center();

        Shape::polygon(vec![
            Point::new(-100., -100.),
            Point::new(0., 100.),
            Point::new(100., -100.),
        ])
        .fill(Fill::new(Color::GREEN))
        .render_at(center, scene)
        .await;

        Shape::circle(Point::new(0., 0.), Points::new(25.))
            .fill(Fill::new(Color::RED))
            .render_at(center, scene)
            .await;

        Ok(())
    }
}
