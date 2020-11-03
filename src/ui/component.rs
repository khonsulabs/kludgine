use crate::{
    math::{Point, PointExt, Raw, Rect, Scaled, Size, SizeExt},
    shape::{Fill, Shape},
    style::{BackgroundColor, ColorPair, FallbackStyle},
    ui::{Context, Entity, Layout, LayoutSolver, LayoutSolverExt, StyledContext},
    window::{
        event::{EventStatus, MouseButton, MouseScrollDelta, TouchPhase},
        CloseResponse,
    },
    KludgineResult,
};
use async_trait::async_trait;
use winit::event::{ElementState, ScanCode, VirtualKeyCode};
mod builder;
mod button;
mod control;
mod image;
mod label;
#[cfg(feature = "ecs")]
pub mod legion;
mod pane;
mod panel;
mod text_field;

pub use self::{
    builder::EntityBuilder,
    button::{Button, ButtonBackgroundColor, ButtonBorder, ButtonPadding, ButtonTextColor},
    control::{
        Border, ComponentBorder, ControlBackgroundColor, ControlBorder, ControlEvent,
        ControlPadding, ControlTextColor,
    },
    image::{
        Image, ImageAlphaAnimation, ImageCommand, ImageFrameAnimation, ImageOptions, ImageScaling,
    },
    label::{Label, LabelBackgroundColor, LabelCommand, LabelTextColor},
    pane::{Pane, PaneBackgroundColor, PaneBorder, PanePadding},
    panel::{
        Panel, PanelBackgroundColor, PanelBorder, PanelCommand, PanelEvent, PanelMessage,
        PanelProvider,
    },
    text_field::{TextField, TextFieldBackgroundColor, TextFieldBorder, TextFieldEvent},
};

pub struct LayoutConstraints {}

#[async_trait]
#[allow(unused_variables)]
pub trait Component: Send + Sync {
    /// Called once the Window is opened
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        Ok(())
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        Ok(Size::new(
            constraints.width.unwrap_or_default(),
            constraints.height.unwrap_or_default(),
        ))
    }

    async fn render(&mut self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        Ok(())
    }

    async fn update(&mut self, context: &mut Context) -> KludgineResult<()> {
        Ok(())
    }

    async fn layout(
        &mut self,
        context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>> {
        let children = context.children().await;
        if children.is_empty() {
            Layout::none().layout()
        } else {
            let mut layout = Layout::absolute();
            for child in children {
                let node = context.arena().get(&child).await.unwrap();
                layout = layout.child(&child, node.bounds().await)?;
            }
            layout.layout()
        }
    }

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        self.render_standard_background::<BackgroundColor, ComponentBorder>(context, layout)
            .await
    }

    async fn render_standard_background<
        C: Into<ColorPair> + FallbackStyle<Raw> + Clone,
        B: Into<ComponentBorder> + FallbackStyle<Raw> + Clone,
    >(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        let bounds = layout.bounds_without_margin();
        if let Some(background) = C::lookup(context.effective_style()) {
            let color_pair = background.clone().into();
            let color = color_pair.themed_color(&context.scene().system_theme().await);

            if color.visible() {
                Shape::rect(bounds)
                    .fill(Fill::new(color))
                    .render_at(Point::default(), context.scene())
                    .await;
            }
        }
        if let Some(border) = B::lookup(context.effective_style()) {
            let border = border.into();
            // TODO the borders should be mitered together rather than drawn overlapping
            if let Some(left) = border.left {
                Shape::rect(Rect::new(
                    bounds.origin,
                    Size::from_lengths(left.width, bounds.size.height()),
                ))
                .fill(Fill::new(
                    left.color
                        .themed_color(&context.scene().system_theme().await),
                ))
                .render_at(Point::default(), context.scene())
                .await;
            }
            if let Some(right) = border.right {
                Shape::rect(Rect::new(
                    Point::from_lengths(bounds.max().x() - right.width, bounds.origin.y()),
                    Size::from_lengths(right.width, bounds.size.height()),
                ))
                .fill(Fill::new(
                    right
                        .color
                        .themed_color(&context.scene().system_theme().await),
                ))
                .render_at(Point::default(), context.scene())
                .await;
            }
            if let Some(top) = border.top {
                Shape::rect(Rect::new(
                    bounds.origin,
                    Size::from_lengths(bounds.size.width(), top.width),
                ))
                .fill(Fill::new(
                    top.color
                        .themed_color(&context.scene().system_theme().await),
                ))
                .render_at(Point::default(), context.scene())
                .await;
            }
            if let Some(bottom) = border.bottom {
                Shape::rect(Rect::new(
                    Point::from_lengths(bounds.origin.x(), bounds.max().y() - bottom.width),
                    Size::from_lengths(bounds.size.width(), bottom.width),
                ))
                .fill(Fill::new(
                    bottom
                        .color
                        .themed_color(&context.scene().system_theme().await),
                ))
                .render_at(Point::default(), context.scene())
                .await;
            }
        }
        Ok(())
    }

    async fn mouse_down(
        &mut self,
        context: &mut Context,
        window_position: Point<f32, Scaled>,
        button: MouseButton,
    ) -> KludgineResult<EventStatus> {
        if self.hit_test(context, window_position).await? {
            context.activate().await;

            Ok(EventStatus::Processed)
        } else {
            Ok(EventStatus::Ignored)
        }
    }

    async fn hovered(&mut self, context: &mut Context) -> KludgineResult<()> {
        Ok(())
    }

    async fn unhovered(&mut self, context: &mut Context) -> KludgineResult<()> {
        Ok(())
    }

    async fn receive_character(
        &mut self,
        context: &mut Context,
        character: char,
    ) -> KludgineResult<()> {
        Ok(())
    }

    async fn keyboard_event(
        &mut self,
        context: &mut Context,
        scancode: ScanCode,
        key: Option<VirtualKeyCode>,
        state: ElementState,
    ) -> KludgineResult<()> {
        Ok(())
    }

    async fn mouse_drag(
        &mut self,
        context: &mut Context,
        window_position: Option<Point<f32, Scaled>>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        let activate = if let Some(window_position) = window_position {
            self.hit_test(context, window_position).await?
        } else {
            false
        };

        if activate {
            context.activate().await;
        } else {
            context.deactivate().await;
        }

        Ok(())
    }

    async fn mouse_up(
        &mut self,
        context: &mut Context,
        window_position: Option<Point<f32, Scaled>>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        if let Some(window_position) = window_position {
            if self.hit_test(context, window_position).await? {
                self.clicked(context, window_position, button).await?
            }
        }
        context.deactivate().await;
        Ok(())
    }

    async fn clicked(
        &mut self,
        context: &mut Context,
        window_position: Point<f32, Scaled>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        Ok(())
    }

    async fn mouse_wheel(
        &mut self,
        context: &mut Context,
        delta: MouseScrollDelta,
        touch_phase: TouchPhase,
    ) -> KludgineResult<EventStatus> {
        Ok(EventStatus::Ignored)
    }

    async fn hit_test(
        &self,
        context: &mut Context,
        window_position: Point<f32, Scaled>,
    ) -> KludgineResult<bool> {
        Ok(context
            .last_layout()
            .await
            .bounds_without_margin()
            .contains(window_position))
    }

    async fn close_requested(&self) -> KludgineResult<CloseResponse> {
        Ok(CloseResponse::Close)
    }
}

#[async_trait]
#[allow(unused_variables)]
pub trait InteractiveComponent: Component {
    type Message: Clone + Send + Sync + std::fmt::Debug + 'static;
    type Command: Clone + Send + Sync + std::fmt::Debug + 'static;
    type Event: Clone + Send + Sync + std::fmt::Debug + 'static;

    async fn receive_message(
        &mut self,
        context: &mut Context,
        message: Self::Message,
    ) -> KludgineResult<()> {
        unimplemented!(
            "Component::receive_message() must be implemented if you're receiving messages"
        )
    }

    async fn receive_command(
        &mut self,
        context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        unimplemented!(
            "Component::receive_message() must be implemented if you're receiving messages"
        )
    }

    fn new_entity<T: InteractiveComponent + 'static>(
        &self,
        context: &mut Context,
        component: T,
    ) -> EntityBuilder<T, Self::Message> {
        context.insert_new_entity(context.index(), component)
    }

    async fn callback(&self, context: &mut Context, message: Self::Event) {
        if let Some(node) = context.arena().get(&context.index()).await {
            node.callback(message).await;
        }
    }
}

pub trait StandaloneComponent: Component {}

impl<T> InteractiveComponent for T
where
    T: StandaloneComponent,
{
    type Message = ();
    type Command = ();
    type Event = ();
}

struct FullyTypedCallback<Command, Event> {
    translator: Box<dyn Fn(Command) -> Event + Send + Sync>,
    target: Context,
}

#[async_trait]
trait TypeErasedCallback<Input>: Send + Sync {
    async fn callback(&self, input: Input);
}

#[async_trait]
impl<Input: Send + 'static, Output: Send + Sync + 'static> TypeErasedCallback<Input>
    for FullyTypedCallback<Input, Output>
{
    async fn callback(&self, input: Input) {
        if let Some(node) = self.target.arena().get(&self.target.index()).await {
            let translated = self.translator.as_ref()(input);
            let component = node.component.write().await;
            component
                .receive_message(&self.target, Box::new(translated))
                .await
        }
    }
}

pub struct Callback<Input> {
    wrapped: Box<dyn TypeErasedCallback<Input>>,
}

impl<Input> Callback<Input>
where
    Input: Send + 'static,
{
    pub fn new<Output: Send + Sync + 'static, F: Fn(Input) -> Output + Send + Sync + 'static>(
        target: Context,
        callback: F,
    ) -> Self {
        Self {
            wrapped: Box::new(FullyTypedCallback {
                translator: Box::new(callback),
                target,
            }),
        }
    }

    pub async fn invoke(&self, input: Input) {
        self.wrapped.callback(input).await
    }
}

pub trait AnimatableComponent: InteractiveComponent + Sized {
    type AnimationFactory;

    fn new_animation_factory(target: Entity<Self>) -> Self::AnimationFactory;
}
