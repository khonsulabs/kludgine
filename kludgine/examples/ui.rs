extern crate kludgine;
use futures::executor::block_on;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(block_on(UIExample::new()));
}

struct UIExample {
    ui: UserInterface,
}

impl UIExample {
    async fn new() -> Self {
        Self {
            ui: Self::create_interface().await.unwrap(),
        }
    }
    async fn create_interface() -> KludgineResult<UserInterface> {
        let grid = Component::new(
            Grid::new(4, 4)
                .with_cell(Point::new(0, 0), Component::new(Interface { message: "A" }))?
                .with_cell(Point::new(1, 0), Component::new(Interface { message: "B" }))?
                .with_cell(Point::new(0, 1), Component::new(Interface { message: "C" }))?
                .with_cell(Point::new(1, 1), Component::new(Interface { message: "D" }))?,
        );
        let ui = UserInterface::new(Style::default());
        ui.set_root(grid).await;
        Ok(ui)
    }
}

impl WindowCreator<UIExample> for UIExample {
    fn window_title() -> String {
        "User Interface - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for UIExample {
    async fn render<'a>(&mut self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        self.ui.render(scene).await?;

        Ok(())
    }

    async fn process_input(&mut self, _event: InputEvent) -> KludgineResult<()> {
        Ok(())
    }
}

#[derive(Debug)]
struct Interface {
    message: &'static str,
}

#[async_trait]
impl Controller for Interface {
    async fn view(&self) -> KludgineResult<Box<dyn View>> {
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
