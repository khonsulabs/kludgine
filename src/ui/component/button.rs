use crate::{
    math::{Point, Raw, Scaled, Size, Surround},
    style::{
        theme::{Classes, Selector},
        Style, StyleComponent,
    },
    ui::{
        component::Component, AbsoluteBounds, Context, ControlEvent, Entity, InteractiveComponent,
        Label, StyledContext,
    },
    window::event::MouseButton,
    KludgineResult,
};
use async_trait::async_trait;
use euclid::Scale;

#[derive(Debug, Clone, Default)]
pub struct ButtonPadding<Unit>(pub Surround<f32, Unit>);

impl StyleComponent<Scaled> for ButtonPadding<Scaled> {
    fn scale(&self, scale: Scale<f32, Scaled, Raw>, destination: &mut Style<Raw>) {
        destination.push(ButtonPadding(self.0 * scale))
    }
}

impl StyleComponent<Raw> for ButtonPadding<Raw> {
    fn scale(&self, _scale: Scale<f32, Raw, Raw>, map: &mut Style<Raw>) {
        map.push(ButtonPadding(self.0));
    }
}

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
        let style_sheet = context.style_sheet().await;

        self.label = self
            .new_entity(context, Label::new(&self.caption))
            .with(Classes::from("button-label"))
            .bounds(AbsoluteBounds::from(
                style_sheet
                    .normal
                    .get_or_default::<ButtonPadding<Scaled>>()
                    .0,
            ))
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
        let padding = context
            .effective_style()
            .get_or_default::<ButtonPadding<Raw>>()
            .0
            / context.scene().scale_factor().await;

        let contraints_minus_padding = padding.inset_constraints(constraints);
        Ok(context
            .content_size(&self.label, &contraints_minus_padding)
            .await?
            + padding.minimum_size())
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
