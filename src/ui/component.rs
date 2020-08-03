use crate::{
    math::{Point, Size},
    scene::SceneTarget,
    shape::{Fill, Shape},
    style::{Style, StyleSheet},
    ui::{
        global_arena, Context, Entity, Index, Layout, LayoutSolver, LayoutSolverExt, Node,
        NodeData, SceneContext, StyledContext, UIState,
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
        constraints: &Size<Option<f32>>,
    ) -> KludgineResult<Size> {
        Ok(Size {
            width: constraints.width.unwrap_or_default(),
            height: constraints.height.unwrap_or_default(),
        })
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
        Layout::none().layout()
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
            context
                .scene()
                .draw_shape(
                    Shape::rect(
                        layout.bounds_without_margin().coord1(),
                        layout.bounds_without_margin().coord2(),
                    )
                    .fill(Fill::Solid(background.into())),
                )
                .await;
        }
        Ok(())
    }

    async fn mouse_down(
        &mut self,
        context: &mut Context,
        window_position: &Point,
        button: MouseButton,
    ) -> KludgineResult<EventStatus> {
        if self.hit_test(context, window_position).await? {
            context.activate().await;

            Ok(EventStatus::Handled)
        } else {
            Ok(EventStatus::Ignored)
        }
    }

    async fn mouse_drag(
        &mut self,
        context: &mut Context,
        window_position: &Option<Point>,
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
        window_position: &Option<Point>,
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
        window_position: &Point,
        button: MouseButton,
    ) -> KludgineResult<()> {
        Ok(())
    }

    async fn hit_test(
        &self,
        context: &mut Context,
        window_position: &Point,
    ) -> KludgineResult<bool> {
        // TODO Should all components actually respond to hit test generically like this?
        Ok(context
            .last_layout()
            .await
            .bounds_without_margin()
            .contains(&window_position))
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
    ) -> EntityBuilder<T> {
        EntityBuilder {
            component,
            scene: context.scene().clone(),
            parent: Some(context.index()),
            style_sheet: Default::default(),
            ui_state: context.ui_state().clone(),
            callback: None,
        }
    }

    async fn send<T: InteractiveComponent + 'static>(&self, target: Entity<T>, message: T::Input) {
        if let Some(target_node) = global_arena().get(target).await {
            let component = target_node.component.read().await;
            if let Some(node_data) = component.as_any().downcast_ref::<NodeData<T>>() {
                node_data
                    .input_sender
                    .send(message)
                    .expect("Error sending to component");
            } else {
                unreachable!("Invalid type in Entity<T> -- Node contained different type than T")
            }
        }
    }

    async fn callback(&self, context: &mut Context, message: Self::Output) {
        let node = context.arena().get(context.index()).await.unwrap();
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
    target: Index,
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
        if let Some(node) = global_arena().get(self.target).await {
            let translated = self.translator.as_ref()(input);
            let component = node.component.write().await;
            component.receive_message(Box::new(translated))
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
        target: Index,
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

pub struct EntityBuilder<C>
where
    C: InteractiveComponent + 'static,
{
    component: C,
    scene: SceneTarget,
    parent: Option<Index>,
    style_sheet: StyleSheet,
    callback: Option<Callback<C::Output>>,
    ui_state: UIState,
}

impl<C> EntityBuilder<C>
where
    C: InteractiveComponent + 'static,
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

    pub fn callback<F: Fn(C::Output) -> O + Send + Sync + 'static, O: Send + Sync + 'static>(
        mut self,
        callback: F,
    ) -> Self {
        self.callback = Some(Callback::new(self.parent.unwrap(), callback));
        self
    }

    pub async fn insert(self) -> KludgineResult<Entity<C>> {
        let index = {
            let node = Node::new(self.component, self.style_sheet, self.callback);
            let index = global_arena().insert(self.parent, node).await;

            let mut context = SceneContext::new(
                index,
                self.scene,
                global_arena().clone(),
                self.ui_state.clone(),
            );
            global_arena()
                .get(index)
                .await
                .unwrap()
                .initialize(&mut context)
                .await?;

            index
        };
        Ok(Entity::new(index))
    }
}
