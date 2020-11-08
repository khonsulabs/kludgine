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
    text_field: Entity<TextField>,
    new_window_button: Entity<Button>,
    current_count: usize,
}

impl WindowCreator for UIExample {
    fn window_title() -> String {
        "User Interface - Kludgine".to_owned()
    }

    fn initial_system_theme() -> SystemTheme {
        SystemTheme::Dark
    }
}

impl Window for UIExample {}

#[derive(Debug, Clone)]
pub enum Message {
    ButtonClicked,
    DialogButtonClicked(DialogChoices),
    NewWindowClicked,
    LabelClicked,
    TextFieldEvent(TextFieldEvent),
}

#[async_trait]
impl InteractiveComponent for UIExample {
    type Message = Message;
    type Command = ();
    type Event = ();

    async fn receive_message(
        &mut self,
        context: &mut Context,
        message: Self::Message,
    ) -> KludgineResult<()> {
        match message {
            Message::LabelClicked => {
                self.current_count += 0;
                self.label
                    .send(LabelCommand::SetValue("You clicked me".to_string()))
                    .await?;
            }
            Message::DialogButtonClicked(clicked) => {
                context
                    .new_layer(Toast::text(format!(
                        "This is a toast. You picked {:?}",
                        clicked
                    )))
                    .bounds(AbsoluteBounds {
                        bottom: Dimension::from_f32(64.),
                        ..Default::default()
                    })
                    .insert()
                    .await?;
            }
            Message::ButtonClicked => {
                self.current_count += 1;
                self.label
                    .send(LabelCommand::SetValue(self.current_count.to_string()))
                    .await?;
                context
                    .new_layer(Dialog::<_, DialogChoices>::text(
                        "This is a dialog... Choose wisely.",
                    ))
                    .with(DialogChoices::buttons())
                    .callback(&self.entity(context), |choice| {
                        Message::DialogButtonClicked(choice.unwrap())
                    })
                    .insert()
                    .await?;
            }
            Message::NewWindowClicked => {
                Runtime::open_window(Self::get_window_builder(), UIExample::default()).await;
            }
            Message::TextFieldEvent(event) => {
                if let TextFieldEvent::ValueChanged(text) = event {
                    self.label
                        .send(LabelCommand::SetValue(text.to_string().await))
                        .await?;
                }
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
            .await
            .style_sheet(Style::new().with(BackgroundColor(Color::new(0.0, 1.0, 1.0, 1.0).into())))
            .bounds(AbsoluteBounds {
                right: Dimension::from_f32(10.),
                bottom: Dimension::from_f32(10.),
                ..Default::default()
            })
            .insert()
            .await?;

        self.text_field = self
            .new_entity(
                context,
                TextField::new(RichText::new(vec![Text::span(
                    "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt",
                    Default::default(),
                )])),
            ).await
            .bounds(AbsoluteBounds {
                left: Dimension::from_f32(32.),
                right: Dimension::from_f32(32.),
                top: Dimension::from_f32(32.),
                ..Default::default()
            })
            .callback(&self.entity(context), Message::TextFieldEvent)
            .insert()
            .await?;

        self.label = self
            .new_entity(context, Label::new("Test Label"))
            .await
            .style_sheet(
                Style::new()
                    .with(ForegroundColor(Color::new(1.0, 1.0, 1.0, 0.1).into()))
                    .with(BackgroundColor(Color::new(1.0, 0.0, 1.0, 0.5).into()))
                    .with(FontSize::new(72.))
                    .with(Alignment::Right),
            )
            .bounds(AbsoluteBounds {
                left: Dimension::from_f32(32.),
                right: Dimension::from_f32(32.),
                top: Dimension::from_f32(96.),
                bottom: Dimension::from_f32(64.),
                ..Default::default()
            })
            .callback(&self.entity(context), |_| Message::LabelClicked)
            .insert()
            .await?;

        self.button = self
            .new_entity(context, Button::new("Press Me"))
            .await
            .normal_style(Style::new().with(BackgroundColor(Color::ROYALBLUE.into())))
            .bounds(AbsoluteBounds {
                bottom: Dimension::from_f32(10.),

                ..Default::default()
            })
            .callback(&self.entity(context), |_| Message::ButtonClicked)
            .insert()
            .await?;

        self.new_window_button = self
            .new_entity(context, Button::new("New Window"))
            .await
            .bounds(AbsoluteBounds {
                bottom: Dimension::from_f32(10.),
                left: Dimension::from_f32(10.),
                ..Default::default()
            })
            .callback(&self.entity(context), |_| Message::NewWindowClicked)
            .insert()
            .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum DialogChoices {
    Agree,
    Cancel,
    MoreInfo,
}

impl DialogChoices {
    fn buttons() -> DialogButtons<Self> {
        DialogButtons(vec![
            DialogButton::default()
                .caption("Proceed")
                .primary()
                .value(Self::Agree)
                .alignment(Alignment::Right),
            DialogButton::default()
                .caption("More Info")
                .value(Self::MoreInfo)
                .alignment(Alignment::Right),
            DialogButton::default()
                .caption("Cancel")
                .cancel()
                .value(Self::Cancel)
                .alignment(Alignment::Left),
        ])
    }
}
