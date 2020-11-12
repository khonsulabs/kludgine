use super::{Component, InteractiveComponent};
use crate::{
    math::{Point, PointExt, Points, Raw, Scaled, Size, SizeExt},
    shape::{Fill, Shape},
    style::{theme::Selector, ColorPair, Style, StyleComponent, UnscaledStyleComponent},
    ui::{Context, Layout, StyledContext},
    KludgineResult,
};
use async_trait::async_trait;
use euclid::{Length, Rect, Scale};

#[derive(Debug)]
pub struct Scrollbar {
    orientation: ScrollbarOrientation,
    metrics: Option<ScrollbarMetrics>,
    offset: Points,

    last_rendered_grip_rect: Option<Rect<f32, Scaled>>,
}

impl Scrollbar {
    pub fn vertical() -> Self {
        Self {
            orientation: ScrollbarOrientation::Vertical,
            metrics: None,
            offset: Points::new(0.),
            last_rendered_grip_rect: None,
        }
    }
    pub fn horizontal() -> Self {
        Self {
            orientation: ScrollbarOrientation::Horizontal,
            metrics: None,
            offset: Points::new(0.),
            last_rendered_grip_rect: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ScrollbarOrientation {
    Vertical,
    Horizontal,
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct ScrollbarMetrics {
    pub content_length: Points,
    pub page_size: Points,
}

#[async_trait]
impl Component for Scrollbar {
    fn classes(&self) -> Option<Vec<Selector>> {
        Some(vec![Selector::from("scrollbar")]) // TODO this should be a control but it's not padded
    }
    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let size = context
            .effective_style()
            .get::<ScrollbarSize<Raw>>()
            .unwrap()
            .0
            / context.scene().scale_factor().await;

        Ok(match self.orientation {
            ScrollbarOrientation::Vertical => {
                Size::new(size.get(), constraints.height.unwrap_or(f32::MAX))
            }
            ScrollbarOrientation::Horizontal => {
                Size::new(constraints.width.unwrap_or(f32::MAX), size.get())
            }
        })
    }

    async fn render(&mut self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        if let Some(metrics) = &self.metrics {
            let size = context
                .effective_style()
                .get::<ScrollbarSize<Raw>>()
                .unwrap()
                .0
                / context.scene().scale_factor().await;
            let grip_color = context
                .effective_style()
                .get::<ScrollbarGripColor>()
                .unwrap()
                .0;
            let bounds = layout.inner_bounds();

            let component_length = match self.orientation {
                ScrollbarOrientation::Horizontal => bounds.size.width(),
                ScrollbarOrientation::Vertical => bounds.size.height(),
            };
            let grip_length = component_length * (metrics.page_size / metrics.content_length);
            let scroll_amount = self
                .offset
                .max(Points::default())
                .min(component_length - grip_length);
            let grip_rect = match self.orientation {
                ScrollbarOrientation::Horizontal => Rect::new(
                    Point::from_lengths(bounds.origin.x() + scroll_amount, bounds.origin.y()),
                    Size::from_lengths(grip_length, size),
                ),
                ScrollbarOrientation::Vertical => Rect::new(
                    Point::from_lengths(bounds.origin.x(), bounds.origin.y() + scroll_amount),
                    Size::from_lengths(size, grip_length),
                ),
            };

            Shape::rect(grip_rect)
                .fill(Fill::new(
                    grip_color.themed_color(&context.scene().system_theme().await),
                ))
                .render_at(Point::default(), context.scene())
                .await;
            self.last_rendered_grip_rect = Some(grip_rect);
        } else {
            self.last_rendered_grip_rect = None;
        }

        Ok(())
    }

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        if self.metrics.is_some() {
            self.render_standard_background(context, layout).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl InteractiveComponent for Scrollbar {
    type Message = ();
    type Command = ScrollbarCommand;
    type Event = ();

    async fn receive_command(
        &mut self,
        context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        match command {
            ScrollbarCommand::SetMetrics(metrics) => {
                if metrics != self.metrics {
                    self.metrics = metrics;
                    context.set_needs_redraw().await;
                }
            }
            ScrollbarCommand::SetOffset(offset) => {
                if self.offset != offset {
                    self.offset = offset;
                    context.set_needs_redraw().await;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum ScrollbarCommand {
    SetOffset(Points),
    SetMetrics(Option<ScrollbarMetrics>),
}

#[derive(Debug, Clone)]
pub struct ScrollbarGripColor(pub ColorPair);

impl UnscaledStyleComponent<Scaled> for ScrollbarGripColor {}

#[derive(Debug, Clone)]
pub struct ScrollbarSize<Unit>(pub Length<f32, Unit>);

impl StyleComponent<Scaled> for ScrollbarSize<Scaled> {
    fn scale(&self, scale: Scale<f32, Scaled, Raw>, map: &mut Style<Raw>) {
        map.push(ScrollbarSize(self.0 * scale));
    }
}

impl StyleComponent<Raw> for ScrollbarSize<Raw> {
    fn scale(&self, _scale: Scale<f32, Raw, Raw>, map: &mut Style<Raw>) {
        map.push(ScrollbarSize(self.0));
    }
}
