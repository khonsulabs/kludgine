use crate::{
    event::MouseButton,
    math::{Point, Size, Surround},
    ui::{
        AbsoluteBounds, Component, Context, Entity, EventStatus, InteractiveComponent, Label,
        Layout, LayoutSolver, LayoutSolverExt, StyledContext,
    },
    KludgineResult,
};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    padding: Surround,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            padding: Surround::uniform(10.),
        }
    }
}

pub struct Button {
    caption: String,
    label: Entity<Label>,
    style: ButtonStyle,
}

#[async_trait]
impl Component for Button {
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        self.label = self
            .new_entity(context, Label::new(&self.caption))
            .insert()
            .await?;
        Ok(())
    }

    async fn mouse_down(
        &mut self,
        _context: &mut Context,
        _position: Point,
        button: MouseButton,
    ) -> KludgineResult<EventStatus> {
        if button == MouseButton::Left {
            Ok(EventStatus::Handled)
        } else {
            Ok(EventStatus::Ignored)
        }
    }

    async fn mouse_up(
        &mut self,
        context: &mut Context,
        position: Option<Point>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        if MouseButton::Left == button {
            let hit = match position {
                Some(position) => self.hit_test(context, position).await?,
                None => false,
            };
            if hit {
                self.callback(context, ButtonEvent::Clicked).await;
            }
        }

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
                    left: crate::math::Dimension::Points(10.),
                    top: crate::math::Dimension::Points(10.),
                    right: crate::math::Dimension::Points(10.),
                    bottom: crate::math::Dimension::Points(10.),
                    ..Default::default()
                },
            )?
            .layout()
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>>,
    ) -> KludgineResult<Size> {
        let contraints_minus_padding = *constraints - self.style.padding.minimum_size();
        Ok(context
            .content_size(self.label, &contraints_minus_padding)
            .await?
            + self.style.padding.minimum_size())
    }
}

impl Button {
    pub fn new(caption: impl ToString) -> Self {
        let caption = caption.to_string();
        Self {
            caption,
            label: Default::default(),
            style: Default::default(),
        }
    }

    pub fn button_style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }
}

#[derive(Clone, Debug)]
pub enum ButtonEvent {
    Clicked,
}

#[derive(Clone, Debug)]
pub enum ButtonCommand {
    SetCaption(String),
    SetButtonStyle(ButtonStyle),
}

#[async_trait]
impl InteractiveComponent for Button {
    type Output = ButtonEvent;
    type Message = ();
    type Input = ButtonCommand;

    async fn receive_input(
        &mut self,
        _context: &mut Context,
        message: Self::Input,
    ) -> KludgineResult<()> {
        match message {
            ButtonCommand::SetCaption(caption) => {
                self.caption = caption;
            }
            ButtonCommand::SetButtonStyle(style) => {
                self.style = style;
            }
        }
        Ok(())
    }
}
