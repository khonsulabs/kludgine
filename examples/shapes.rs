extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(Shapes::default());
}

#[derive(Default)]
struct Shapes;

impl WindowCreator<Shapes> for Shapes {
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
            Point::new(Points::from_f32(-100.), Points::from_f32(-100.)),
            Point::new(Points::from_f32(0.), Points::from_f32(100.)),
            Point::new(Points::from_f32(100.), Points::from_f32(-100.)),
        ])
        .fill(Fill::new(Color::GREEN))
        .draw_at(center, context.scene())
        .await;

        Shape::circle(
            Point::new(Points::from_f32(0.), Points::from_f32(0.)),
            Points::from_f32(25.),
        )
        .fill(Fill::new(Color::RED))
        .draw_at(center, context.scene())
        .await;

        Ok(())
    }
}
