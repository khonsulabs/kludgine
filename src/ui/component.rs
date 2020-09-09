use crate::{
    math::{Point, Scaled, Size},
    scene::SceneTarget,
    shape::{Fill, Shape},
    style::{Style, StyleSheet},
    ui::{
        AbsoluteBounds, Context, Entity, HierarchicalArena, Index, Layout, LayoutSolver,
        LayoutSolverExt, Node, SceneContext, StyledContext, UIState,
    },
    window::InputEvent,
    KludgineResult,
};
use async_trait::async_trait;
use winit::event::MouseButton;

pub struct LayoutConstraints {}

#[async_trait]
#[allow(unused_variables)]
pub trait Component: Send + Sync {
    /// Called once the Window is opened
    async fn initialize(&mut self, context: &mut SceneContext) -> KludgineResult<()> {
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

    async fn render(&self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        Ok(())
    }

    async fn update(&mut self, context: &mut SceneContext) -> KludgineResult<()> {
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

    async fn process_input(
        &mut self,
        context: &mut Context,
        event: InputEvent,
    ) -> KludgineResult<()> {
        Ok(())
    }

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        if let Some(background) = context.effective_style().background_color {
            Shape::rect(layout.bounds_without_margin())
                .fill(Fill::new(background))
                .draw_at(Point::default(), context.scene())
                .await;
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

            Ok(EventStatus::Handled)
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

pub enum EventStatus {
    Ignored,
    Handled,
}

#[async_trait]
#[allow(unused_variables)]
pub trait InteractiveComponent: Component {
    type Message: Clone + Send + Sync + std::fmt::Debug + 'static;
    type Input: Clone + Send + Sync + std::fmt::Debug + 'static;
    type Output: Clone + Send + Sync + std::fmt::Debug + 'static;

    async fn receive_message(
        &mut self,
        context: &mut Context,
        message: Self::Message,
    ) -> KludgineResult<()> {
        unimplemented!(
            "Component::receive_message() must be implemented if you're receiving messages"
        )
    }

    async fn receive_input(
        &mut self,
        context: &mut Context,
        command: Self::Input,
    ) -> KludgineResult<()> {
        unimplemented!(
            "Component::receive_message() must be implemented if you're receiving messages"
        )
    }

    fn new_entity<T: InteractiveComponent + 'static>(
        &self,
        context: &mut SceneContext,
        component: T,
    ) -> EntityBuilder<T, Self::Message> {
        EntityBuilder {
            component,
            scene: context.scene().clone(),
            parent: Some(context.index()),
            interactive: true,
            ui_state: context.ui_state().clone(),
            arena: context.arena().clone(),
            style_sheet: Default::default(),
            bounds: Default::default(),
            callback: None,
            _marker: Default::default(),
        }
    }

    async fn callback(&self, context: &mut Context, message: Self::Output) {
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
    type Input = ();
    type Output = ();
}

struct FullyTypedCallback<Input, Output> {
    translator: Box<dyn Fn(Input) -> Output + Send + Sync>,
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
            component.receive_message(&self.target, Box::new(translated))
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
    component: C,
    scene: SceneTarget,
    parent: Option<Index>,
    style_sheet: StyleSheet,
    bounds: AbsoluteBounds,
    interactive: bool,
    callback: Option<Callback<C::Output>>,
    ui_state: UIState,
    arena: HierarchicalArena,
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> EntityBuilder<C, P>
where
    C: InteractiveComponent + 'static,
    P: Send + Sync + 'static,
{
    pub fn style_sheet(mut self, sheet: StyleSheet) -> Self {
        self.style_sheet = sheet;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style_sheet.normal = style;
        self
    }

    pub fn hover(mut self, style: Style) -> Self {
        self.style_sheet.hover = style;
        self
    }

    pub fn active(mut self, style: Style) -> Self {
        self.style_sheet.active = style;
        self
    }

    pub fn focus(mut self, style: Style) -> Self {
        self.style_sheet.focus = style;
        self
    }

    pub fn bounds(mut self, bounds: AbsoluteBounds) -> Self {
        self.bounds = bounds;
        self
    }

    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    pub fn callback<F: Fn(C::Output) -> P + Send + Sync + 'static>(mut self, callback: F) -> Self {
        let target = Context::new(
            self.parent.unwrap(),
            self.arena.clone(),
            self.ui_state.clone(),
        );
        self.callback = Some(Callback::new(target, callback));
        self
    }

    pub async fn insert(self) -> KludgineResult<Entity<C>> {
        let index = {
            let node = Node::new(
                self.component,
                self.style_sheet,
                self.bounds,
                self.interactive,
                self.callback,
            );
            let index = self.arena.insert(self.parent, node).await;

            let mut context =
                SceneContext::new(index, self.scene, self.arena.clone(), self.ui_state.clone());
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
        )))
    }
}

pub trait AnimatableComponent: InteractiveComponent + Sized {
    type AnimationFactory;

    fn new_animation_factory(target: Entity<Self>) -> Self::AnimationFactory;
}
