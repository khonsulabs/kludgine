extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(UIExample {
        image: None,
        label: None,
    });
}

struct UIExample {
    image: Option<Entity<Image>>,
    label: Option<Entity<Label>>,
}

impl WindowCreator<UIExample> for UIExample {
    fn window_title() -> String {
        "User Interface - Kludgine".to_owned()
    }
}

impl Window for UIExample {}

impl StandaloneComponent for UIExample {}

#[async_trait]
impl Component for UIExample {
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        let sprite = include_aseprite_sprite!("assets/stickguy").await?;
        self.image = Some(
            self.new_entity(context, Image::new(sprite))
                .style(Style {
                    background_color: Some(Color::new(0.0, 1.0, 1.0, 1.0)),
                    ..Default::default()
                })
                .insert()
                .await?,
        );

        self.label = Some(
            self.new_entity(context, Label::new("Test Label"))
                .style(Style {
                    color: Some(Color::GREEN),
                    background_color: Some(Color::new(1.0, 0.0, 1.0, 0.5)),
                    font_size: Some(72.),
                    ..Default::default()
                })
                .insert()
                .await?,
        );

        Ok(())
    }

    async fn render(&self, _context: &mut StyledContext, _layout: &Layout) -> KludgineResult<()> {
        // self.ui.render(scene).await?;

        Ok(())
    }

    async fn layout(
        &mut self,
        _context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>> {
        Layout::absolute()
            .child(
                self.label.unwrap(),
                AbsoluteBounds {
                    left: Dimension::Points(32.),
                    right: Dimension::Points(64.),
                    top: Dimension::Points(32.),
                    bottom: Dimension::Points(64.),
                    ..Default::default()
                },
            )?
            .child(
                self.image.unwrap(),
                AbsoluteBounds {
                    right: Dimension::Points(10.),
                    bottom: Dimension::Points(10.),
                    ..Default::default()
                },
            )?
            .layout()
    }

    async fn process_input(
        &mut self,
        _context: &mut Context,
        event: InputEvent,
    ) -> KludgineResult<()> {
        if let Event::MouseButton { .. } = event.event {
            UIExample::open(UIExample {
                image: None,
                label: None,
            })
            .await;
        }
        Ok(())
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
