extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(UIExample::default());
}

#[derive(Default)]
struct UIExample {
    image: Entity<Image>,
    label: Entity<Label>,
    button: Entity<Button>,
    new_window_button: Entity<Button>,
    current_count: usize,
}

impl WindowCreator<UIExample> for UIExample {
    fn window_title() -> String {
        "User Interface - Kludgine".to_owned()
    }
}

impl Window for UIExample {}

#[derive(Debug, Clone)]
pub enum Message {
    ButtonClicked,
    NewWindowClicked,
}

#[async_trait]
impl InteractiveComponent for UIExample {
    type Message = Message;
    type Input = ();
    type Output = ();

    async fn receive_message(
        &mut self,
        _context: &mut Context,
        message: Self::Message,
    ) -> KludgineResult<()> {
        match message {
            Message::ButtonClicked => {
                self.current_count += 1;
                self.send(
                    self.label,
                    LabelCommand::SetValue(self.current_count.to_string()),
                )
                .await;
            }
            Message::NewWindowClicked => {
                Runtime::open_window(Self::get_window_builder(), UIExample::default()).await;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Component for UIExample {
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        let sprite = include_aseprite_sprite!("assets/stickguy").await?;
        self.image = self
            .new_entity(context, Image::new(sprite))
            .style(Style {
                background_color: Some(Color::new(0.0, 1.0, 1.0, 1.0)),
                ..Default::default()
            })
            .insert()
            .await?;

        self.label = self
            .new_entity(context, Label::new("Test Label"))
            .style(Style {
                color: Some(Color::GREEN),
                background_color: Some(Color::new(1.0, 0.0, 1.0, 0.5)),
                font_size: Some(72.),
                alignment: Some(Alignment::Right),
                ..Default::default()
            })
            .insert()
            .await?;

        self.button = self
            .new_entity(context, Button::new("Press Me"))
            .style(Style {
                font_size: Some(16.),
                color: Some(Color::BLACK),
                background_color: Some(Color::new(0.7, 0.7, 0.7, 1.0)),
                ..Default::default()
            })
            .hover(Style {
                background_color: Some(Color::new(0.8, 0.8, 0.8, 1.0)),
                ..Default::default()
            })
            .active(Style {
                color: Some(Color::WHITE),
                background_color: Some(Color::new(0.4, 0.4, 0.4, 1.0)),
                ..Default::default()
            })
            .callback(|_| Message::ButtonClicked)
            .insert()
            .await?;

        self.new_window_button = self
            .new_entity(context, Button::new("New Window"))
            .style(Style {
                font_size: Some(16.),
                color: Some(Color::BLACK),
                background_color: Some(Color::new(0.7, 0.7, 0.7, 1.0)),
                ..Default::default()
            })
            .hover(Style {
                background_color: Some(Color::new(0.8, 0.8, 0.8, 1.0)),
                ..Default::default()
            })
            .active(Style {
                color: Some(Color::WHITE),
                background_color: Some(Color::new(0.4, 0.4, 0.4, 1.0)),
                ..Default::default()
            })
            .callback(|_| Message::NewWindowClicked)
            .insert()
            .await?;

        Ok(())
    }

    async fn layout(
        &mut self,
        _context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>> {
        Layout::absolute()
            .child(
                self.label,
                AbsoluteBounds {
                    left: Dimension::Points(32.),
                    right: Dimension::Points(32.),
                    top: Dimension::Points(32.),
                    bottom: Dimension::Points(64.),
                    ..Default::default()
                },
            )?
            .child(
                self.image,
                AbsoluteBounds {
                    right: Dimension::Points(10.),
                    bottom: Dimension::Points(10.),
                    ..Default::default()
                },
            )?
            .child(
                self.button,
                AbsoluteBounds {
                    bottom: Dimension::Points(10.),

                    ..Default::default()
                },
            )?
            .child(
                self.new_window_button,
                AbsoluteBounds {
                    bottom: Dimension::Points(10.),
                    left: Dimension::Points(10.),
                    ..Default::default()
                },
            )?
            .layout()
    }
}
