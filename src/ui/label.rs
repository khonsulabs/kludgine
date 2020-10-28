use crate::{
    math::{Point, PointExt, Points, Raw, Scaled, Size, SizeExt},
    style::{
        Alignment, ColorPair, GenericStyle, Style, UnscaledFallbackStyle, UnscaledStyleComponent,
        VerticalAlignment,
    },
    text::{wrap::TextWrap, Text},
    ui::{
        control::ControlBorder, Component, Context, ControlBackgroundColor, ControlEvent,
        ControlTextColor, InteractiveComponent, Layout, StyledContext,
    },
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
            VerticalAlignment::Bottom => todo!(),
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

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        self.render_standard_background::<LabelBackgroundColor, ControlBorder>(context, layout)
            .await
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

#[derive(Debug, Clone)]
pub struct LabelBackgroundColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for LabelBackgroundColor {
    fn unscaled_should_be_inherited(&self) -> bool {
        false
    }
}

impl Default for LabelBackgroundColor {
    fn default() -> Self {
        Self(ControlBackgroundColor::default().0)
    }
}

impl UnscaledFallbackStyle for LabelBackgroundColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style.get::<Self>().cloned().or_else(|| {
            ControlBackgroundColor::lookup_unscaled(style).map(|fg| LabelBackgroundColor(fg.0))
        })
    }
}

impl Into<ColorPair> for LabelBackgroundColor {
    fn into(self) -> ColorPair {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct LabelTextColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for LabelTextColor {}

impl Default for LabelTextColor {
    fn default() -> Self {
        Self(ControlTextColor::default().0)
    }
}

impl UnscaledFallbackStyle for LabelTextColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlTextColor::lookup_unscaled(style).map(|fg| LabelTextColor(fg.0)))
    }
}

impl Into<ColorPair> for LabelTextColor {
    fn into(self) -> ColorPair {
        self.0
    }
}
