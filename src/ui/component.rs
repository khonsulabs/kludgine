use crate::{
    math::Size,
    shape::{Fill, Shape},
    style::Style,
    ui::{
        global_arena, Context, Entity, Index, Layout, LayoutSolver, LayoutSolverExt, Node,
        NodeData, SceneContext, StyledContext,
    },
    window::InputEvent,
    KludgineResult,
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
                    .fill(Fill::Solid(background)),
                )
                .await;
        }
        Ok(())
    }
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
        message: Self::Input,
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
        EntityBuilder {
            component,
            parent: Some(context.index()),
            style: Default::default(),
            hover_style: Default::default(),
            active_style: Default::default(),
            focus_style: Default::default(),
            callback: None,
        }
    }

    async fn send<T: InteractiveComponent + 'static, O: 'static>(
        &self,
        target: Entity<T, Self::Message>,
        message: T::Input,
    ) {
        if let Some(target_node) = global_arena().get(target).await {
            let component = target_node.component.read().await;
            if let Some(node_data) = component.as_any().downcast_ref::<NodeData<T, O>>() {
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
        let node = global_arena().get(context.index()).await.unwrap();
        node.callback(message).await
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

pub struct Callback<Input, Output> {
    translator: Box<dyn Fn(Input) -> Output + Send + Sync>,
    target: Index,
}

impl<Input, Output> Callback<Input, Output>
where
    Output: Send + 'static,
{
    pub async fn invoke(&self, input: Input) {
        if let Some(node) = global_arena().get(self.target).await {
            let translated = self.translator.as_ref()(input);
            let component = node.component.write().await;
            component.receive_message(Box::new(translated))
        }
    }
}

pub struct EntityBuilder<C, O>
where
    C: InteractiveComponent + 'static,
{
    component: C,
    parent: Option<Index>,
    style: Style,
    hover_style: Style,
    active_style: Style,
    focus_style: Style,
    callback: Option<Callback<C::Output, O>>,
}

impl<C, O> EntityBuilder<C, O>
where
    C: InteractiveComponent + 'static,
    O: Send + 'static,
{
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    pub fn hover(mut self, style: Style) -> Self {
        self.hover_style = style;
        self
    }
    pub fn active(mut self, style: Style) -> Self {
        self.active_style = style;
        self
    }
    pub fn focus(mut self, style: Style) -> Self {
        self.focus_style = style;
        self
    }

    pub fn callback<F: Fn(C::Output) -> O + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.callback = Some(Callback {
            translator: Box::new(callback),
            target: self.parent.unwrap(),
        });
        self
    }

    pub async fn insert(self) -> KludgineResult<Entity<C, O>> {
        let index = {
            let node = Node::new(
                self.component,
                self.style,
                self.hover_style,
                self.active_style,
                self.focus_style,
                self.callback,
            );
            let index = global_arena().insert(self.parent, node).await;

            let mut context = Context::new(index);
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
