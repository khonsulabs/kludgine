use crate::{
    math::{Point, PointExt, Points, Raw, Scaled, Size, SizeExt},
    style::{theme::Selector, Alignment, Style, VerticalAlignment},
    text::{wrap::TextWrap, Text},
    ui::{Component, Context, ControlEvent, InteractiveComponent, Layout, StyledContext},
    window::event::MouseButton,
    KludgineResult,
};
use async_trait::async_trait;

#[derive(Debug)]
pub struct Label {
    value: String,
}

#[derive(Clone, Debug)]
pub enum LabelCommand {
    SetValue(String),
}

#[async_trait]
impl InteractiveComponent for Label {
    type Command = LabelCommand;
    type Message = ();
    type Event = ControlEvent;

    async fn receive_command(
        &mut self,
        context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        match command {
            LabelCommand::SetValue(new_value) => {
                if self.value != new_value {
                    self.value = new_value;
                    context.set_needs_redraw().await;
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Component for Label {
    fn classes(&self) -> Option<Vec<Selector>> {
        Some(vec![Selector::from("label")])
    }

    async fn update(&mut self, _context: &mut Context) -> KludgineResult<()> {
        Ok(())
    }

    async fn render(&mut self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        let inner_bounds = layout.inner_bounds();
        let scale = context.scene().scale_factor().await;

        let text = self.create_text(context.effective_style());
        let wrapped = text
            .wrap(
                context.scene(),
                self.wrapping(
                    &inner_bounds.size,
                    context.effective_style().get_or_default::<Alignment>(),
                ),
            )
            .await?;
        let wrapped_size = wrapped.size().await;

        let vertical_alignment = context
            .effective_style()
            .get_or_default::<VerticalAlignment>();
        let location = match vertical_alignment {
            VerticalAlignment::Top => inner_bounds.origin,
            VerticalAlignment::Center => Point::from_lengths(
                inner_bounds.origin.x(),
                inner_bounds.origin.y()
                    + (inner_bounds.size.height() - wrapped_size.height() / scale) / 2.,
            ),
            VerticalAlignment::Bottom => Point::from_lengths(
                inner_bounds.origin.x(),
                inner_bounds.origin.y() + inner_bounds.size.height()
                    - wrapped_size.height() / scale,
            ),
        };

        wrapped
            .render(context.scene(), location, true)
            .await
            .map(|_| ())
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let text = self.create_text(context.effective_style());
        let wrapping = self.wrapping(
            &Size::new(
                constraints.width.unwrap_or_else(|| f32::MAX),
                constraints.height.unwrap_or_else(|| f32::MAX),
            ),
            context.effective_style().get_or_default::<Alignment>(),
        );
        let wrapped_size = text.wrap(context.scene(), wrapping).await?.size().await;
        Ok(wrapped_size / context.scene().scale_factor().await)
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

    async fn hit_test(
        &self,
        context: &mut Context,
        window_position: Point<f32, Scaled>,
    ) -> KludgineResult<bool> {
        if self.has_callback(context).await {
            Ok(self
                .last_layout(context)
                .await
                .inner_bounds()
                .contains(window_position))
        } else {
            Ok(false)
        }
    }
}

impl Label {
    pub fn new(value: impl ToString) -> Self {
        Self {
            value: value.to_string(),
        }
    }
    fn create_text(&self, effective_style: &Style<Raw>) -> Text {
        Text::span(&self.value, effective_style.clone())
    }

    fn wrapping(&self, size: &Size<f32, Scaled>, alignment: Alignment) -> TextWrap {
        TextWrap::SingleLine {
            max_width: Points::new(size.width),
            truncate: true,
            alignment,
        }
    }
}
