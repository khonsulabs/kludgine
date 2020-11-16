use crate::{
    math::{Point, PointExt, Raw, Rect, Scaled, Size, SizeExt, Surround},
    shape::{Fill, Shape},
    style::{theme::Selector, BackgroundColor, ColorPair, StyleComponent},
    ui::{Context, Entity, Layout, LayoutSolver, LayoutSolverExt, StyledContext},
    window::{
        event::{EventStatus, MouseButton, MouseScrollDelta, TouchPhase},
        CloseResponse,
    },
    KludgineResult,
};
use async_handle::Handle;
use async_trait::async_trait;
use generational_arena::Index;
use winit::event::{ElementState, ScanCode, VirtualKeyCode};
mod builder;
mod button;
mod control;
mod dialog;
mod grid;
mod image;
mod label;
#[cfg(feature = "ecs")]
pub mod legion;
mod list;
mod pane;
mod panel;
mod pending;
mod scroll;
mod scrollbar;
mod text_field;
mod toast;

pub use self::{
    builder::EntityBuilder,
    button::Button,
    control::{Border, ComponentBorder, ComponentPadding, ContentOffset, ControlEvent},
    dialog::{Dialog, DialogButton, DialogButtonSpacing, DialogButtons},
    grid::{Grid, GridCommand, GridEvent},
    image::{
        Image, ImageAlphaAnimation, ImageCommand, ImageFrameAnimation, ImageOptions, ImageScaling,
    },
    label::{Label, LabelCommand},
    list::{List, ListCommand, ListEvent},
    pane::Pane,
    panel::{Panel, PanelCommand, PanelEvent, PanelMessage, PanelProvider},
    scroll::{ComponentOverflow, Overflow, Scroll, ScrollCommand, ScrollEvent, ScrollGutterColor},
    scrollbar::{Scrollbar, ScrollbarCommand, ScrollbarGripColor, ScrollbarMetrics, ScrollbarSize},
    text_field::{TextField, TextFieldEvent},
    toast::Toast,
};

pub struct LayoutConstraints {}

#[async_trait]
#[allow(unused_variables)]
pub trait Component: Send + Sync {
    fn classes(&self) -> Option<Vec<Selector>> {
        None
    }

    /// Called once the Window is opened
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        Ok(())
    }

    async fn content_size_with_padding(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<(Size<f32, Scaled>, Surround<f32, Scaled>)> {
        let padding = context
            .effective_style()
            .get_or_default::<ComponentPadding<Raw>>()
            .0
            / context.scene().scale_factor().await;

        let constraints_minus_padding = padding.inset_constraints(constraints);
        Ok((
            self.content_size(context, &constraints_minus_padding)
                .await?,
            padding,
        ))
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
        self.standard_layout(context).await
    }

    async fn standard_layout(
        &mut self,
        context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>> {
        let children = self.children(context).await;
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

    async fn render_standard_background<Background, Border>(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()>
    where
        Background: StyleComponent<Raw> + Clone + Into<ColorPair>,
        Border: StyleComponent<Raw> + Clone + Into<ComponentBorder>,
    {
        let bounds = layout.bounds_without_margin();
        if let Some(background) = context.effective_style().get::<Background>() {
            let color_pair = background.clone().into();
            let color = color_pair.themed_color(&context.scene().system_theme().await);

            if color.visible() {
                Shape::rect(bounds)
                    .fill(Fill::new(color))
                    .render_at(Point::default(), context.scene())
                    .await;
            }
        }
        if let Some(border) = context.effective_style().get::<Border>() {
            let border = border.clone().into();
            // TODO the borders should be mitered together rather than drawn overlapping
            if let Some(left) = &border.left {
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
            if let Some(right) = &border.right {
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
            if let Some(top) = &border.top {
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
            if let Some(bottom) = &border.bottom {
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
            context.activate(context.layer_index().await).await;

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
            context.activate(context.layer_index().await).await;
        } else {
            context.deactivate(context.layer_index().await).await;
        }

        Ok(())
    }

    async fn mouse_up(
        &mut self,
        context: &mut Context,
        window_position: Option<Point<f32, Scaled>>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        context.deactivate(context.layer_index().await).await;

        if let Some(window_position) = window_position {
            if self.hit_test(context, window_position).await? {
                self.clicked(context, window_position, button).await?
            }
        }

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
        Ok(self
            .last_layout(context)
            .await
            .bounds_without_margin()
            .contains(window_position))
    }

    async fn close_requested(&self) -> KludgineResult<CloseResponse> {
        Ok(CloseResponse::Close)
    }

    async fn last_layout(&self, context: &mut Context) -> Layout {
        context.last_layout_for(context.index()).await
    }

    async fn children(&self, context: &mut Context) -> Vec<Index> {
        context.children_of(context.index()).await
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

    async fn new_entity<T: InteractiveComponent + 'static>(
        &self,
        context: &mut Context,
        component: T,
    ) -> EntityBuilder<T, Self::Message> {
        context.insert_new_entity(context.index(), component).await
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

#[async_trait]
pub trait InteractiveComponentExt: Sized {
    async fn component<T: Send + Sync + 'static>(&self, context: &mut Context)
        -> Option<Handle<T>>;
    fn entity(&self, context: &mut Context) -> Entity<Self>;
    async fn activate(&self, context: &mut Context);
    async fn deactivate(&self, context: &mut Context);
}

#[async_trait]
impl<C> InteractiveComponentExt for C
where
    C: InteractiveComponent + 'static,
{
    async fn component<T: Send + Sync + 'static>(
        &self,
        context: &mut Context,
    ) -> Option<Handle<T>> {
        context.get_component_from(context.entity::<Self>()).await
    }

    fn entity(&self, context: &mut Context) -> Entity<C> {
        context.entity()
    }

    async fn activate(&self, context: &mut Context) {
        context.activate(context.entity::<Self>()).await
    }

    async fn deactivate(&self, context: &mut Context) {
        context.deactivate(context.entity::<Self>()).await
    }
}
