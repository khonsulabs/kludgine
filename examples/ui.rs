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
        // let grid = Component::new(
        //     Grid::new(4, 4)
        //         .with_cell(
        //             Point::new(0, 0),
        //             Component::new(Interface { click_count: 0 }),
        //         )?
        //         .with_cell(
        //             Point::new(1, 0),
        //             Component::new(Interface { click_count: 0 }),
        //         )?
        //         .with_cell(
        //             Point::new(0, 1),
        //             Component::new(Interface { click_count: 0 }),
        //         )?
        //         .with_cell(
        //             Point::new(1, 1),
        //             Component::new(Interface { click_count: 0 }),
        //         )?,
        // );
        let ui = UserInterface::new(Style::default());
        // ui.set_root(grid).await;
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
    async fn initialize(&mut self, _scene: &mut Scene) -> KludgineResult<()> {
        let sprite = include_aseprite_sprite!("assets/stickguy").await?;
        self.ui.new_entity(Image::new(sprite)).insert().await?;
        Ok(())
    }

    async fn update<'a>(&mut self, scene: &SceneTarget) -> KludgineResult<()> {
        self.ui.update(scene).await
    }

    async fn render<'a>(&self, scene: &SceneTarget) -> KludgineResult<()> {
        self.ui.render(scene).await?;

        Ok(())
    }

    async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        self.ui.process_input(event).await.map(|_| ())
    }
}

// #[derive(Debug)]
// struct Interface {
//     click_count: i32,
// }

// #[async_trait]
// impl Controller for Interface {
//     async fn view(&self) -> KludgineResult<KludgineHandle<Box<dyn View>>> {
//         Label::default()
//             .with_value(self.click_count.to_string())
//             .with_style(Style {
//                 font_size: Some(60.0),
//                 color: Some(Color::new(0.0, 0.5, 0.5, 1.0)),
//                 ..Default::default()
//             })
//             .with_hover_style(Style {
//                 color: Some(Color::new(1.0, 1.0, 1.0, 1.0)),
//                 ..Default::default()
//             })
//             .with_padding(Surround::uniform(Dimension::Auto))
//             .build()
//     }

//     async fn mouse_button_down(
//         &mut self,
//         _component: &Component,
//         _button: MouseButton,
//         __window_position: Point,
//     ) -> KludgineResult<ComponentEventStatus> {
//         self.click_count += 1;
//         Ok(ComponentEventStatus::rebuild_view_processed())
//     }
// }
