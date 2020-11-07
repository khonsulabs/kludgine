use crate::{
    math::{Point, Scaled, Size, Surround},
    style::{theme::Selector, Style},
    ui::{
        component::{Component, ComponentPadding},
        Context, ControlEvent, Entity, InteractiveComponent, Label, StyledContext,
    },
    window::event::MouseButton,
    KludgineResult,
};
use async_trait::async_trait;

#[derive(Debug)]
pub struct Button {
    caption: String,
    label: Entity<Label>,
}

#[async_trait]
impl Component for Button {
    fn classes(&self) -> Option<Vec<Selector>> {
        Some(vec![Selector::from("button"), Selector::from("control")])
    }

    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        self.label = self
            .new_entity(context, Label::new(&self.caption))
            .with_class("clear-background")
            .await
            .style_sheet(Style::default().with(ComponentPadding::<Scaled>(Surround::default())))
            .interactive(false)
            .insert()
            .await?;

        Ok(())
    }

    async fn clicked(
        &mut self,
        context: &mut Context,
        window_position: Point<f32, Scaled>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        self.callback(
            context,
            ControlEvent::Clicked {
                button,
                window_position,
            },
        )
        .await;
        Ok(())
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let (content_size, padding) = context
            .content_size_with_padding(&self.label, &constraints)
            .await?;
        Ok(content_size + padding.minimum_size())
    }
}

impl Button {
    pub fn new(caption: impl ToString) -> Self {
        let caption = caption.to_string();
        Self {
            caption,
            label: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ButtonCommand {
    SetCaption(String),
}

#[async_trait]
impl InteractiveComponent for Button {
    type Event = ControlEvent;
    type Message = ();
    type Command = ButtonCommand;

    async fn receive_command(
        &mut self,
        _context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        match command {
            ButtonCommand::SetCaption(caption) => {
                self.caption = caption;
            }
        }
        Ok(())
    }
}
