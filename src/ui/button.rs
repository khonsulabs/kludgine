use crate::{
    event::MouseButton,
    math::Point,
    ui::{
        AbsoluteBounds, Component, Context, Entity, EventStatus, InteractiveComponent, Label,
        Layout, LayoutSolver, LayoutSolverExt, StyledContext,
    },
    KludgineResult,
};
use async_trait::async_trait;

pub struct Button {
    caption: String,
    label: Entity<Label>,
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
pub enum ButtonEvent {
    Clicked,
}

impl InteractiveComponent for Button {
    type Output = ButtonEvent;
    type Message = ();
    type Input = ();
}
