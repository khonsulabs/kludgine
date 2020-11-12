use super::{
    pending::PendingComponent, InteractiveComponent, InteractiveComponentExt, Scrollbar,
    ScrollbarCommand, ScrollbarMetrics,
};
use crate::{
    math::{Point, PointExt, Points, Raw, Rect, Scaled, Size, SizeExt, Surround, Vector},
    prelude::EventStatus,
    style::{theme::Selector, UnscaledStyleComponent},
    ui::{
        component::Component, AbsoluteBounds, Context, Entity, Indexable, Layout, LayoutContext,
        LayoutSolver, LayoutSolverExt, StyledContext,
    },
    KludgineResult,
};
use approx::relative_ne;
use async_handle::Handle;
use async_trait::async_trait;
use generational_arena::Index;
use std::fmt::Debug;
use winit::event::{MouseScrollDelta, TouchPhase};

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
}

#[derive(Debug)]
pub struct Scroll<C>
where
    C: InteractiveComponent + 'static,
{
    contents: PendingComponent<C>,
    last_overflow: Handle<(Option<Points>, Option<Points>)>,
    scroll: Vector<f32, Scaled>,
    horizontal_scrollbar: Entity<Scrollbar>,
    vertical_scrollbar: Entity<Scrollbar>,
}

impl<C> Scroll<C>
where
    C: InteractiveComponent + 'static,
{
    pub fn new(component: C) -> Self {
        Self {
            contents: PendingComponent::Pending(component),
            last_overflow: Handle::new((None, None)),
            scroll: Default::default(),
            horizontal_scrollbar: Default::default(),
            vertical_scrollbar: Default::default(),
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
                    .await
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
            .await
            .bounds(
                AbsoluteBounds::default()
                    .with_bottom(Points::new(0.))
                    .with_left(Points::new(0.))
                    .with_right(Points::new(0.)),
            )
            .insert()
            .await?;
        self.vertical_scrollbar = self
            .new_entity(context, Scrollbar::vertical())
            .await
            .bounds(
                AbsoluteBounds::default()
                    .with_right(Points::new(0.))
                    .with_bottom(Points::new(0.))
                    .with_top(Points::new(0.)),
            )
            .insert()
            .await?;

        Ok(())
    }

    async fn layout(
        &mut self,
        _context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>> {
        ScrollLayout {
            last_overflow: self.last_overflow.clone(),
            contents: self.contents.entity().index(),
            scroll: self.scroll,
            horizontal_scrollbar: self.horizontal_scrollbar.clone(),
            vertical_scrollbar: self.vertical_scrollbar.clone(),
        }
        .layout()
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
        let overflow = self.last_overflow.read().await;
        println!(
            "Scroll amount: {:?}, current scroll: {:?}, current overflow: {:?}",
            scroll_amount, self.scroll, *overflow
        );
        if let Some(horizontal_overflow) = overflow.0 {
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

        if let Some(vertical_overflow) = overflow.1 {
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
        let ScrollMessage::ChildEvent(message) = message;
        self.callback(context, ScrollEvent::Child(message)).await;
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
    last_overflow: Handle<(Option<Points>, Option<Points>)>,
    contents: Index,
    scroll: Vector<f32, Scaled>,
    horizontal_scrollbar: Entity<Scrollbar>,
    vertical_scrollbar: Entity<Scrollbar>,
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
            .effective_style()
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

            let overflow = calculated_content_size.to_vector() - inner_bounds.size.to_vector();
            let overflow = (
                if overflow.x > 0. {
                    Some(overflow.x())
                } else {
                    None
                },
                if overflow.y > 0. {
                    Some(overflow.y())
                } else {
                    None
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
                let mut last_overflow = self.last_overflow.write().await;
                if *last_overflow != overflow {
                    *last_overflow = overflow;

                    self.horizontal_scrollbar
                        .send(ScrollbarCommand::SetMetrics(overflow.0.map(|_| {
                            ScrollbarMetrics {
                                content_length: calculated_content_size.width(),
                                page_size: inner_bounds.size.width(),
                            }
                        })))
                        .await?;

                    self.vertical_scrollbar
                        .send(ScrollbarCommand::SetMetrics(overflow.1.map(|_| {
                            ScrollbarMetrics {
                                content_length: calculated_content_size.height(),
                                page_size: inner_bounds.size.height(),
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
                            calculated_content_size.max(inner_bounds.size),
                        ),
                        margin: Default::default(),
                        padding,
                        content_offset: Some(-new_scroll),
                        clip_to: inner_bounds,
                    },
                )
                .await;

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
        }
        Ok(())
    }
}
