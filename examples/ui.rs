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

impl WindowCreator for UIExample {
    fn window_title() -> String {
        "User Interface - Kludgine".to_owned()
    }
}

impl Window for UIExample {}

#[derive(Debug, Clone)]
pub enum Message {
    ButtonClicked,
    NewWindowClicked,
    LabelClicked,
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
            Message::LabelClicked => {
                self.current_count += 0;
                self.label
                    .send(LabelCommand::SetValue("You clicked me".to_string()))
                    .await?;
            }
            Message::ButtonClicked => {
                self.current_count += 1;
                self.label
                    .send(LabelCommand::SetValue(self.current_count.to_string()))
                    .await?;
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
    async fn initialize(&mut self, context: &mut SceneContext) -> KludgineResult<()> {
        let sprite = include_aseprite_sprite!("assets/stickguy").await?;
        self.image = self
            .new_entity(context, Image::new(sprite))
            .style(Style {
                background_color: Some(Color::new(0.0, 1.0, 1.0, 1.0)),
                ..Default::default()
            })
            .bounds(AbsoluteBounds {
                right: Dimension::from_f32(10.),
                bottom: Dimension::from_f32(10.),
                ..Default::default()
            })
            .insert()
            .await?;

        self.label = self
            .new_entity(context, Label::new("Test Label"))
            .style(Style {
                color: Some(Color::new(1.0, 1.0, 1.0, 0.1)),
                background_color: Some(Color::new(1.0, 0.0, 1.0, 0.5)),
                font_size: Some(Points::new(72.)),
                alignment: Some(Alignment::Right),
                ..Default::default()
            })
            .bounds(AbsoluteBounds {
                left: Dimension::from_f32(32.),
                right: Dimension::from_f32(32.),
                top: Dimension::from_f32(32.),
                bottom: Dimension::from_f32(64.),
                ..Default::default()
            })
            .callback(|_| Message::LabelClicked)
            .insert()
            .await?;

        self.button = self
            .new_entity(context, Button::new("Press Me"))
            .style(Style {
                color: Some(Color::ROYALBLUE),
                ..Default::default()
            })
            .bounds(AbsoluteBounds {
                bottom: Dimension::from_f32(10.),

                ..Default::default()
            })
            .callback(|_| Message::ButtonClicked)
            .insert()
            .await?;

        self.new_window_button = self
            .new_entity(context, Button::new("New Window"))
            .bounds(AbsoluteBounds {
                bottom: Dimension::from_f32(10.),
                left: Dimension::from_f32(10.),
                ..Default::default()
            })
            .callback(|_| Message::NewWindowClicked)
            .insert()
            .await?;

        Ok(())
    }
}
