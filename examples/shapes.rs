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

impl Window for Shapes {}

impl StandaloneComponent for Shapes {}

#[async_trait]
impl Component for Shapes {
    async fn render(&self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        let center = layout.bounds_without_margin().center();

        Shape::polygon(vec![
            Point::new(-100., -100.),
            Point::new(0., 100.),
            Point::new(100., -100.),
        ])
        .fill(Fill::new(Color::GREEN))
        .render_at(center, context.scene())
        .await;

        Shape::circle(Point::new(0., 0.), Points::new(25.))
            .fill(Fill::new(Color::RED))
            .render_at(center, context.scene())
            .await;

        Ok(())
    }
}
