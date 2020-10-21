use crate::{
    color::Color,
    event::{MouseButton, MouseScrollDelta, TouchPhase},
    math::{Point, Raw, Scaled, Size},
    scene::Scene,
    shape::{Fill, Shape},
    style::{BackgroundColor, FallbackStyle, Style, StyleSheet},
    ui::{
        node::ThreadsafeAnyMap, AbsoluteBounds, Context, Entity, HierarchicalArena, Index, Layout,
        LayoutSolver, LayoutSolverExt, Node, StyledContext, UIState,
    },
    window::EventStatus,
    Handle, KludgineResult,
};
use async_trait::async_trait;

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
        render_background::<BackgroundColor>(context, layout).await
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
        let component = Handle::new(component);
        let mut components = ThreadsafeAnyMap::new();
        components.insert(component);
        EntityBuilder {
            components,
            scene: context.scene().clone(),
            parent: Some(context.index()),
            interactive: true,
            ui_state: context.ui_state().clone(),
            arena: context.arena().clone(),
            style_sheet: Default::default(),
            callback: None,
            _marker: Default::default(),
        }
    }

    async fn callback(&self, context: &mut Context, message: Self::Event) {
        let node = context.arena().get(&context.index()).await.unwrap();
        node.callback(message).await;
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

pub struct EntityBuilder<C, P>
where
    C: InteractiveComponent + 'static,
{
    components: ThreadsafeAnyMap,
    scene: Scene,
    parent: Option<Index>,
    style_sheet: StyleSheet,
    interactive: bool,
    callback: Option<Callback<C::Event>>,
    ui_state: UIState,
    arena: HierarchicalArena,
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> EntityBuilder<C, P>
where
    C: InteractiveComponent + 'static,
    P: Send + Sync + 'static,
{
    pub fn style_sheet<S: Into<StyleSheet>>(mut self, sheet: S) -> Self {
        self.style_sheet = sheet.into();
        self
    }

    pub fn normal_style(mut self, style: Style<Scaled>) -> Self {
        self.style_sheet.normal = style;
        self
    }

    pub fn hover(mut self, style: Style<Scaled>) -> Self {
        self.style_sheet.hover = style;
        self
    }

    pub fn active(mut self, style: Style<Scaled>) -> Self {
        self.style_sheet.active = style;
        self
    }

    pub fn focus(mut self, style: Style<Scaled>) -> Self {
        self.style_sheet.focus = style;
        self
    }

    pub fn bounds(mut self, bounds: AbsoluteBounds) -> Self {
        self.components.insert(Handle::new(bounds));
        self
    }

    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    pub fn callback<F: Fn(C::Event) -> P + Send + Sync + 'static>(mut self, callback: F) -> Self {
        let target = Context::new(
            self.parent.unwrap(),
            self.arena.clone(),
            self.ui_state.clone(),
            self.scene.clone(),
        );
        self.callback = Some(Callback::new(target, callback));
        self
    }

    pub async fn insert(mut self) -> KludgineResult<Entity<C>> {
        let theme = self.scene.theme().await;
        self.components.insert(Handle::new(
            self.style_sheet.inherit_from(&theme.default_style_sheet()),
        ));
        let index = {
            let node = Node::from_components::<C>(self.components, self.interactive, self.callback);
            let index = self.arena.insert(self.parent, node).await;

            let mut context = Context::new(
                index,
                self.arena.clone(),
                self.ui_state.clone(),
                self.scene.clone(),
            );
            self.arena
                .get(&index)
                .await
                .unwrap()
                .initialize(&mut context)
                .await?;

            index
        };
        Ok(Entity::new(Context::new(
            index,
            self.arena.clone(),
            self.ui_state,
            self.scene.clone(),
        )))
    }
}

pub trait AnimatableComponent: InteractiveComponent + Sized {
    type AnimationFactory;

    fn new_animation_factory(target: Entity<Self>) -> Self::AnimationFactory;
}

pub async fn render_background<C: Into<Color> + FallbackStyle<Raw> + Clone>(
    context: &mut StyledContext,
    layout: &Layout,
) -> KludgineResult<()> {
    if let Some(background) = C::lookup(context.effective_style()) {
        Shape::rect(layout.bounds_without_margin())
            .fill(Fill::new(background.clone().into()))
            .render_at(Point::default(), context.scene())
            .await;
    }
    Ok(())
}
