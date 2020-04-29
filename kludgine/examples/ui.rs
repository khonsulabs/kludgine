extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::<UIExample>::default().run();
}

#[derive(Default)]
struct UIExample {}

impl WindowCreator<UIExample> for UIExample {
    fn window_title() -> String {
        "Text - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for UIExample {
    fn render(&mut self, scene: &mut SceneTarget) -> KludgineResult<()> {
        let ui = UserInterface::new(Style::default());
        let grid = Component::new(
            Grid::new(4, 4)
                .with_cell(Point::new(0, 0), Component::new(Interface { message: "A" }))?
                .with_cell(Point::new(1, 0), Component::new(Interface { message: "B" }))?
                .with_cell(Point::new(0, 1), Component::new(Interface { message: "C" }))?
                .with_cell(Point::new(1, 1), Component::new(Interface { message: "D" }))?,
        );
        ui.set_root(grid);
        ui.render(scene)?;

        Ok(())
    }
}

#[derive(Debug)]
struct Interface {
    message: &'static str,
}

impl Controller for Interface {
    fn view(&self) -> KludgineResult<Box<dyn View>> {
        Label::default()
            .with_value(self.message)
            .with_style(Style {
                font_size: Some(60.0),
                color: Some(Color::new(0.0, 0.5, 0.5, 1.0)),
                ..Default::default()
            })
            .with_padding(Surround::uniform(Dimension::Auto))
            .with_margin(Surround {
                left: Dimension::Points(50.0),
                ..Default::default()
            })
            .build()
    }
}
