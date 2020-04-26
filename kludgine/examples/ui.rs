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
        let component = ui.create_component(Interface {});
        ui.set_root(&component);
        ui.render(scene)?;

        Ok(())
    }
}

struct Interface {}

impl Controller for Interface {
    fn view(&self) -> KludgineResult<Box<dyn View>> {
        Label::default()
            .with_value("Hello, World!")
            .with_style(Style {
                font_size: Some(60.0),
                color: Some(Color::new(0.0, 0.5, 0.5, 1.0)),
                ..Default::default()
            })
            .with_padding(Surround::uniform(Dimension::Auto))
            .build()
    }
}
