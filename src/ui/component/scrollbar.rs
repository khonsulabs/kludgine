use std::time::{Duration, Instant};

use super::{Component, InteractiveComponent};
use crate::{
    math::{Point, PointExt, Points, Raw, Scaled, Size, SizeExt},
    prelude::EventStatus,
    shape::{Fill, Shape},
    style::{theme::Selector, ColorPair, Style, StyleComponent, UnscaledStyleComponent},
    ui::{Context, Layout, StyledContext},
    KludgineResult,
};
use async_trait::async_trait;
use euclid::{Length, Rect, Scale};
use winit::event::MouseButton;

#[derive(Debug)]
pub struct Scrollbar {
    orientation: ScrollbarOrientation,
    metrics: Option<ScrollbarMetrics>,
    offset: Points,

    last_rendered_grip_rect: Rect<f32, Scaled>,
    mouse_state: Option<ScrollbarMouseState>,
}

impl Scrollbar {
    pub fn vertical() -> Self {
        Self {
            orientation: ScrollbarOrientation::Vertical,
            metrics: None,
            offset: Points::new(0.),
            last_rendered_grip_rect: Default::default(),
            mouse_state: None,
        }
    }
    pub fn horizontal() -> Self {
        Self {
            orientation: ScrollbarOrientation::Horizontal,
            metrics: None,
            offset: Points::new(0.),
            last_rendered_grip_rect: Default::default(),
            mouse_state: None,
        }
    }

    async fn set_offset(&mut self, offset: Points, context: &mut Context) {
        self.offset = offset;
        self.callback(context, ScrollbarEvent::OffsetChanged(self.offset))
            .await;
        context.set_needs_redraw().await;
    }

    async fn page_up(&mut self, context: &mut Context) {
        if let Some(metrics) = &self.metrics {
            let new_offset = (self.offset - metrics.page_size).max(Points::default());
            self.set_offset(new_offset, context).await;
        }
    }

    async fn page_down(&mut self, context: &mut Context) {
        if let Some(metrics) = &self.metrics {
            let new_offset = (self.offset + metrics.page_size).min(metrics.content_length);
            self.set_offset(new_offset, context).await;
        }
    }

    fn compute_mouse_information(
        &self,
        window_position: Point<f32, Scaled>,
        bounds: &Rect<f32, Scaled>,
    ) -> MouseInfo {
        match self.orientation {
            ScrollbarOrientation::Horizontal => MouseInfo {
                mouse_location: window_position.x(),
                origin: bounds.origin.x(),
                grip_start: self.last_rendered_grip_rect.origin.x(),
                grip_length: self.last_rendered_grip_rect.size.width(),
                total_length: bounds.size.width(),
            },
            ScrollbarOrientation::Vertical => MouseInfo {
                mouse_location: window_position.y(),
                origin: bounds.origin.y(),
                grip_start: self.last_rendered_grip_rect.origin.y(),
                grip_length: self.last_rendered_grip_rect.size.height(),
                total_length: bounds.size.height(),
            },
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
            let scroll_percent =
                Points::new(self.offset.0 / (metrics.content_length.0 - metrics.page_size.0));
            let remaining_bar = Points::new(component_length.0 - grip_length.0);
            let scroll_amount = Points::new(scroll_percent.0 * remaining_bar.0);

            let scroll_amount = scroll_amount
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
            self.last_rendered_grip_rect = grip_rect;
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

    async fn mouse_down(
        &mut self,
        context: &mut Context,
        window_position: Point<f32, Scaled>,
        button: MouseButton,
    ) -> KludgineResult<EventStatus> {
        let bounds = self.last_layout(context).await.inner_bounds();
        let info = self.compute_mouse_information(window_position, &bounds);

        if info.mouse_location < info.origin
            || info.mouse_location > info.origin + info.total_length
        {
            Ok(EventStatus::Ignored)
        } else if info.mouse_location < info.grip_start {
            self.page_up(context).await;
            self.mouse_state = Some(ScrollbarMouseState::Paging {
                up: true,
                last_page: Instant::now(),
            });
            Ok(EventStatus::Processed)
        } else if info.mouse_location < info.grip_start + info.grip_length {
            self.mouse_state = Some(ScrollbarMouseState::Dragging {
                button,
                starting_mouse_location: info.mouse_location,
                starting_grip_start: info.grip_start,
            });
            Ok(EventStatus::Processed)
        } else {
            self.page_down(context).await;
            self.mouse_state = Some(ScrollbarMouseState::Paging {
                up: false,
                last_page: Instant::now(),
            });
            Ok(EventStatus::Processed)
        }
    }

    async fn mouse_drag(
        &mut self,
        context: &mut Context,
        window_position: Option<Point<f32, Scaled>>,
        dragged_button: MouseButton,
    ) -> KludgineResult<()> {
        if let Some(mouse_state) = self.mouse_state {
            if let Some(window_position) = window_position {
                if let Some(metrics) = &self.metrics {
                    let bounds = self.last_layout(context).await.inner_bounds();
                    let info = self.compute_mouse_information(window_position, &bounds);

                    match mouse_state {
                        ScrollbarMouseState::Paging { up, last_page } => {
                            // TODO This properly limits the frequency of paging, but unfortunately winit will only send events when the mouse moves. We probably need to move some of this logic into Update and just use this to trigger a redraw at the right moment.
                            let next_moment =
                                last_page.checked_add(Duration::from_millis(250)).unwrap();
                            let now = Instant::now();
                            if now >= next_moment {
                                if up {
                                    if info.mouse_location < info.grip_start {
                                        self.page_up(context).await;
                                    }
                                } else if info.mouse_location > info.grip_start + info.grip_length {
                                    self.page_down(context).await;
                                }
                                self.mouse_state =
                                    Some(ScrollbarMouseState::Paging { up, last_page: now });
                            }
                        }
                        ScrollbarMouseState::Dragging {
                            button,
                            starting_mouse_location,
                            starting_grip_start,
                        } => {
                            if dragged_button == button {
                                let delta = info.mouse_location - starting_mouse_location;
                                let new_grip_start = starting_grip_start + delta;
                                let remaining_bar = info.total_length - info.grip_length;
                                let scrollable_amount =
                                    (metrics.content_length.0 - metrics.page_size.0);
                                let offset_per_bar_pixel = scrollable_amount / remaining_bar.0;
                                let offset = (new_grip_start - info.origin) * offset_per_bar_pixel;
                                self.set_offset(offset, context).await;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn mouse_up(
        &mut self,
        _context: &mut Context,
        _window_position: Option<Point<f32, Scaled>>,
        _button: MouseButton,
    ) -> KludgineResult<()> {
        self.mouse_state = None;
        Ok(())
    }
}

#[async_trait]
impl InteractiveComponent for Scrollbar {
    type Message = ();
    type Command = ScrollbarCommand;
    type Event = ScrollbarEvent;

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
pub enum ScrollbarEvent {
    OffsetChanged(Points),
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

#[derive(Debug, Copy, Clone)]
enum ScrollbarMouseState {
    Paging {
        up: bool,
        last_page: Instant,
    },
    Dragging {
        button: MouseButton,
        starting_mouse_location: Points,
        starting_grip_start: Points,
    },
}

struct MouseInfo {
    mouse_location: Points,
    origin: Points,
    grip_start: Points,
    grip_length: Points,
    total_length: Points,
}
