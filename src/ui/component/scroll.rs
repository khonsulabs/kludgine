use crate::{
    math::{Point, PointExt, Points, Raw, Rect, Scaled, Size, SizeExt, Surround, Vector},
    prelude::EventStatus,
    style::{theme::Selector, ColorPair, UnscaledStyleComponent},
    ui::{
        component::{
            pending::PendingComponent, scrollbar::ScrollbarEvent, Component, InteractiveComponent,
            InteractiveComponentExt, Scrollbar, ScrollbarCommand, ScrollbarMetrics,
        },
        Context, Entity, Indexable, Layout, LayoutContext, LayoutSolver, LayoutSolverExt,
        StyledContext,
    },
    KludgineResult,
};
use approx::relative_ne;
use async_handle::Handle;
use async_trait::async_trait;
use generational_arena::Index;
use std::fmt::Debug;
use winit::event::{MouseScrollDelta, TouchPhase};
mod gutter;

#[derive(Debug, Clone)]
pub enum ScrollEvent<E> {
    Child(E),
}
#[derive(Debug, Clone)]
pub enum ScrollCommand<CC> {
    Child(CC),
}

#[derive(Debug, Clone)]
pub enum ScrollMessage<E> {
    ChildEvent(E),
    HorizontalScrollbarScrolled(Points),
    VerticalScrollbarScrolled(Points),
}

#[derive(Debug)]
pub struct Scroll<C>
where
    C: InteractiveComponent + 'static,
{
    contents: PendingComponent<C>,
    render_info: Handle<RenderInfo>,
    scroll: Vector<f32, Scaled>,
    horizontal_scrollbar: Entity<Scrollbar>,
    vertical_scrollbar: Entity<Scrollbar>,
    gutter: Entity<gutter::Gutter>,
}

#[derive(Default, Debug)]
struct RenderInfo {
    overflow: (Option<Points>, Option<Points>),
    effective_scrollbar_size: Size<f32, Scaled>,
}

impl<C> Scroll<C>
where
    C: InteractiveComponent + 'static,
{
    pub fn new(component: C) -> Self {
        Self {
            contents: PendingComponent::Pending(component),
            render_info: Handle::default(),
            scroll: Default::default(),
            horizontal_scrollbar: Default::default(),
            vertical_scrollbar: Default::default(),
            gutter: Default::default(),
        }
    }
}

#[async_trait]
impl<C> Component for Scroll<C>
where
    C: InteractiveComponent + 'static,
{
    fn classes(&self) -> Option<Vec<Selector>> {
        Some(vec![Selector::from("scroll")])
    }

    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        if let PendingComponent::Pending(contents) = std::mem::replace(
            &mut self.contents,
            PendingComponent::Entity(Default::default()),
        ) {
            self.contents = PendingComponent::Entity(
                self.new_entity(context, contents)
                    .await?
                    .callback(&self.entity(context), |e: C::Event| {
                        ScrollMessage::ChildEvent(e)
                    })
                    .insert()
                    .await?,
            );
        } else {
            unreachable!("A component should never be re-initialized");
        }

        self.horizontal_scrollbar = self
            .new_entity(context, Scrollbar::horizontal())
            .await?
            .callback(&self.entity(context), |evt| {
                let ScrollbarEvent::OffsetChanged(new_offset) = evt;
                ScrollMessage::HorizontalScrollbarScrolled(new_offset)
            })
            .insert()
            .await?;
        self.vertical_scrollbar = self
            .new_entity(context, Scrollbar::vertical())
            .await?
            .callback(&self.entity(context), |evt| {
                let ScrollbarEvent::OffsetChanged(new_offset) = evt;
                ScrollMessage::VerticalScrollbarScrolled(new_offset)
            })
            .insert()
            .await?;

        self.gutter = self
            .new_entity(context, gutter::Gutter)
            .await?
            .insert()
            .await?;

        Ok(())
    }

    async fn layout(
        &mut self,
        _context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>> {
        ScrollLayout {
            render_info: self.render_info.clone(),
            contents: self.contents.entity().index(),
            scroll: self.scroll,
            horizontal_scrollbar: self.horizontal_scrollbar.clone(),
            vertical_scrollbar: self.vertical_scrollbar.clone(),
            gutter: self.gutter.clone(),
        }
        .layout()
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let (content_size, padding) = context
            .content_size_with_padding(&self.contents.entity(), &constraints)
            .await?;
        let child_size = content_size + padding.minimum_size();
        Ok(Size::new(
            child_size
                .width
                .min(constraints.width.unwrap_or(child_size.width)),
            child_size
                .height
                .min(constraints.height.unwrap_or(child_size.height)),
        ))
    }

    async fn mouse_wheel(
        &mut self,
        context: &mut Context,
        delta: MouseScrollDelta,
        _touch_phase: TouchPhase,
    ) -> KludgineResult<EventStatus> {
        let scroll_amount = -match delta {
            // TODO change line delta to query something?
            MouseScrollDelta::LineDelta(x, y) => Vector::new(x * 20., y * 20.),
            MouseScrollDelta::PixelDelta(delta) => {
                Vector::<f64, Raw>::new(delta.x, delta.y).to_f32()
                    / context.scene().scale_factor().await
            }
        };

        let mut status = EventStatus::Ignored;
        let render_info = self.render_info.read().await;
        if let Some(horizontal_overflow) = render_info.overflow.0 {
            if relative_ne!(scroll_amount.x, 0.) {
                let target_scroll = (self.scroll.x() + scroll_amount.x())
                    .min(horizontal_overflow)
                    .max(Points::default());
                if relative_ne!(target_scroll.0, self.scroll.x) {
                    status = EventStatus::Processed;
                    self.scroll.set_x(target_scroll);
                    let _ = self
                        .horizontal_scrollbar
                        .send(ScrollbarCommand::SetOffset(target_scroll))
                        .await;
                }
            }
        }

        if let Some(vertical_overflow) = render_info.overflow.1 {
            if relative_ne!(scroll_amount.y, 0.) {
                let target_scroll = (self.scroll.y() + scroll_amount.y())
                    .min(vertical_overflow)
                    .max(Points::default());
                if relative_ne!(target_scroll.0, self.scroll.y) {
                    status = EventStatus::Processed;
                    self.scroll.set_y(target_scroll);
                    let _ = self
                        .vertical_scrollbar
                        .send(ScrollbarCommand::SetOffset(target_scroll))
                        .await;
                }
            }
        }

        if matches!(&status, EventStatus::Processed) {
            context.set_needs_redraw().await;
        }

        Ok(status)
    }
}

#[async_trait]
impl<C> InteractiveComponent for Scroll<C>
where
    C: InteractiveComponent + 'static,
{
    type Message = ScrollMessage<C::Event>;
    type Command = ScrollCommand<C::Command>;
    type Event = ScrollEvent<C::Event>;

    async fn receive_command(
        &mut self,
        _context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        let ScrollCommand::Child(command) = command;
        self.contents.entity().send(command).await
    }

    async fn receive_message(
        &mut self,
        context: &mut Context,
        message: Self::Message,
    ) -> KludgineResult<()> {
        match message {
            ScrollMessage::ChildEvent(message) => {
                self.callback(context, ScrollEvent::Child(message)).await;
            }
            ScrollMessage::HorizontalScrollbarScrolled(new_offset) => {
                self.scroll.set_x(new_offset);
                context.set_needs_redraw().await;
            }
            ScrollMessage::VerticalScrollbarScrolled(new_offset) => {
                self.scroll.set_y(new_offset);
                context.set_needs_redraw().await;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Overflow {
    Clip,
    Scroll,
}

impl Default for Overflow {
    fn default() -> Self {
        Overflow::Clip
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ComponentOverflow {
    pub horizontal: Overflow,
    pub vertical: Overflow,
}

impl ComponentOverflow {
    pub fn can_scroll(&self) -> bool {
        matches!(&self.horizontal, Overflow::Scroll) || matches!(&self.vertical, Overflow::Scroll)
    }
}

impl UnscaledStyleComponent<Scaled> for ComponentOverflow {}

#[derive(Debug)]
struct ScrollLayout {
    render_info: Handle<RenderInfo>,
    contents: Index,
    scroll: Vector<f32, Scaled>,
    horizontal_scrollbar: Entity<Scrollbar>,
    vertical_scrollbar: Entity<Scrollbar>,
    gutter: Entity<gutter::Gutter>,
}

#[async_trait]
impl LayoutSolver for ScrollLayout {
    async fn layout_within(
        &self,
        bounds: &Rect<f32, Scaled>,
        _content_size: &Size<f32, Scaled>,
        padding: &Surround<f32, Scaled>,
        context: &LayoutContext,
    ) -> KludgineResult<()> {
        let overflow = context
            .effective_style()?
            .get_or_default::<ComponentOverflow>();
        let inner_bounds = padding.inset_rect(bounds);

        if let Some(node) = context.arena().get(&self.contents).await {
            let constrained_size = Size::new(
                match overflow.horizontal {
                    Overflow::Scroll => None,
                    Overflow::Clip => Some(inner_bounds.width()),
                },
                match overflow.vertical {
                    Overflow::Scroll => None,
                    Overflow::Clip => Some(inner_bounds.height()),
                },
            );
            let (content_size, padding) = context
                .content_size_with_padding(&self.contents, &constrained_size)
                .await?;
            let bounds = node.bounds().await;

            let content_size_with_padding = content_size + padding.minimum_size();
            let calculated_content_size = Size::from_lengths(
                bounds
                    .width
                    .length()
                    .unwrap_or_else(|| content_size_with_padding.width()),
                bounds
                    .height
                    .length()
                    .unwrap_or_else(|| content_size_with_padding.height()),
            );

            let horizontal_scrollbar_size = context
                .content_size(
                    &self.horizontal_scrollbar,
                    &Size::new(
                        Some(inner_bounds.size.width),
                        Some(inner_bounds.size.height),
                    ),
                )
                .await?;
            let vertical_scrollbar_size = context
                .content_size(
                    &self.vertical_scrollbar,
                    &Size::new(
                        Some(inner_bounds.size.width),
                        Some(inner_bounds.size.height),
                    ),
                )
                .await?;

            let scrollbar_size = Size::from_lengths(
                vertical_scrollbar_size.width(),
                horizontal_scrollbar_size.height(),
            );
            let effective_scrollbar_size = Size::from_lengths(
                if calculated_content_size.height() > inner_bounds.size.height() {
                    scrollbar_size.width()
                } else {
                    Points::default()
                },
                if calculated_content_size.width() > inner_bounds.size.width() {
                    scrollbar_size.height()
                } else {
                    Points::default()
                },
            );

            let inner_content_size = inner_bounds.size - effective_scrollbar_size;
            let overflow = calculated_content_size - inner_content_size;
            let overflow = (
                if overflow.width > 0. {
                    Some(overflow.width())
                } else {
                    None
                },
                if overflow.height > 0. {
                    Some(overflow.height())
                } else {
                    None
                },
            );

            // The above logic could have caused an edge case where one scrollbar needs to be shown, and the presence of that scrollbar causes the other scollbar to become visible. To avoid this edge case, we now trust overflow for containing the truth of whether a scrollbar will be shown.
            let effective_scrollbar_size = Size::from_lengths(
                if overflow.0.is_some() {
                    scrollbar_size.width()
                } else {
                    Points::default()
                },
                if overflow.1.is_some() {
                    scrollbar_size.height()
                } else {
                    Points::default()
                },
            );

            let new_scroll = Vector::new(
                self.scroll
                    .x
                    .min(overflow.0.unwrap_or_default().get())
                    .max(0.),
                self.scroll
                    .y
                    .min(overflow.1.unwrap_or_default().get())
                    .max(0.),
            );

            {
                let mut render_info = self.render_info.write().await;
                if render_info.overflow != overflow {
                    render_info.effective_scrollbar_size = effective_scrollbar_size;
                    render_info.overflow = overflow;

                    self.horizontal_scrollbar
                        .send(ScrollbarCommand::SetMetrics(overflow.0.map(|overflow| {
                            ScrollbarMetrics {
                                content_length: overflow
                                    + inner_content_size.width()
                                    + effective_scrollbar_size.width(),
                                page_size: inner_content_size.width(),
                            }
                        })))
                        .await?;

                    self.vertical_scrollbar
                        .send(ScrollbarCommand::SetMetrics(overflow.1.map(|overflow| {
                            ScrollbarMetrics {
                                content_length: overflow
                                    + inner_content_size.height()
                                    + effective_scrollbar_size.width(),
                                page_size: inner_content_size.height(),
                            }
                        })))
                        .await?;
                }
            }

            context
                .insert_layout(
                    self.contents,
                    Layout {
                        bounds: Rect::new(
                            inner_bounds.origin,
                            (calculated_content_size + effective_scrollbar_size)
                                .max(inner_content_size),
                        ),
                        margin: Surround {
                            right: effective_scrollbar_size.width(),
                            bottom: effective_scrollbar_size.height(),
                            ..Default::default()
                        },
                        padding,
                        content_offset: Some(-new_scroll),
                        clip_to: inner_bounds,
                    },
                )
                .await;

            if overflow.0.is_some() {
                let width_correction = if overflow.1.is_some() {
                    vertical_scrollbar_size.width()
                } else {
                    Points::default()
                };
                let bounds = Rect::new(
                    Point::from_lengths(
                        inner_bounds.origin.x(),
                        inner_bounds.origin.y() + inner_bounds.size.height()
                            - horizontal_scrollbar_size.height(),
                    ),
                    Size::from_lengths(
                        horizontal_scrollbar_size.width() - width_correction,
                        horizontal_scrollbar_size.height(),
                    ),
                );
                context
                    .insert_layout(
                        self.horizontal_scrollbar.index(),
                        Layout {
                            clip_to: bounds,
                            bounds,
                            ..Default::default()
                        },
                    )
                    .await;
            }

            if overflow.1.is_some() {
                let height_correction = if overflow.0.is_some() {
                    horizontal_scrollbar_size.height()
                } else {
                    Points::default()
                };
                let bounds = Rect::new(
                    Point::from_lengths(
                        inner_bounds.origin.x() + inner_bounds.size.width()
                            - vertical_scrollbar_size.width(),
                        inner_bounds.origin.y(),
                    ),
                    Size::from_lengths(
                        vertical_scrollbar_size.width(),
                        vertical_scrollbar_size.height() - height_correction,
                    ),
                );
                context
                    .insert_layout(
                        self.vertical_scrollbar.index(),
                        Layout {
                            clip_to: bounds,
                            bounds,
                            ..Default::default()
                        },
                    )
                    .await;
            }

            if effective_scrollbar_size.area() > 0. {
                let bounds = Rect::new(
                    inner_bounds.max() - effective_scrollbar_size,
                    effective_scrollbar_size,
                );
                context
                    .insert_layout(
                        self.gutter.index(),
                        Layout {
                            clip_to: bounds,
                            bounds,
                            ..Default::default()
                        },
                    )
                    .await;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ScrollGutterColor(pub ColorPair);

impl Into<ColorPair> for ScrollGutterColor {
    fn into(self) -> ColorPair {
        self.0
    }
}

impl UnscaledStyleComponent<Scaled> for ScrollGutterColor {}
