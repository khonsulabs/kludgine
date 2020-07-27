use crate::{
    event::MouseButton,
    math::{Point, Size},
    runtime::Runtime,
    style::Style,
    ui::{
        Callback, Component, Context, EventStatus, InteractiveComponent, Layout, LayoutSolver,
        SceneContext, StyledContext,
    },
    window::InputEvent,
    KludgineHandle, KludgineResult,
};
use async_trait::async_trait;
use std::any::Any;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub(crate) trait AnyNode: PendingEventProcessor + Component {
    fn as_any(&self) -> &dyn Any;
    fn style(&self) -> &'_ Style;
    fn hover_style(&self) -> &'_ Style;
    fn focus_style(&self) -> &'_ Style;
    fn active_style(&self) -> &'_ Style;
    fn set_layout(&mut self, layout: Layout);
    fn get_layout(&self) -> &'_ Layout;
    fn receive_message(&self, message: Box<dyn Any>);
}

#[async_trait]
pub(crate) trait PendingEventProcessor {
    async fn process_pending_events(&mut self, context: &mut Context) -> KludgineResult<()>;
    async fn send_callback(&self, input: Box<dyn Any + Send + Sync>);
}

impl<T: InteractiveComponent + 'static> AnyNode for NodeData<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn style(&self) -> &'_ Style {
        &self.style
    }

    fn hover_style(&self) -> &'_ Style {
        &self.hover_style
    }

    fn focus_style(&self) -> &'_ Style {
        &self.focus_style
    }

    fn active_style(&self) -> &'_ Style {
        &self.active_style
    }

    fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
    }

    fn get_layout(&self) -> &'_ Layout {
        &self.layout
    }

    fn receive_message(&self, message: Box<dyn Any>) {
        let message = message.downcast_ref::<T::Message>().unwrap().clone();
        let _ = self.message_sender.send(message);
    }
}

#[async_trait]
impl<T: InteractiveComponent + 'static> PendingEventProcessor for NodeData<T> {
    async fn process_pending_events(&mut self, context: &mut Context) -> KludgineResult<()> {
        while let Ok(message) = self.input_receiver.try_recv() {
            self.component.receive_input(context, message).await?
        }
        while let Ok(message) = self.message_receiver.try_recv() {
            self.component.receive_message(context, message).await?
        }
        Ok(())
    }

    async fn send_callback(&self, output: Box<dyn Any + Send + Sync>) {
        let output = output.downcast_ref::<T::Output>().unwrap().clone();
        if let Some(callback) = self.callback.as_ref() {
            callback.invoke(output).await;
        }
    }
}

pub struct NodeData<T>
where
    T: InteractiveComponent,
{
    pub(crate) component: T,
    callback: Option<Callback<T::Output>>,
    pub(crate) style: Style,
    pub(crate) hover_style: Style,
    pub(crate) active_style: Style,
    pub(crate) focus_style: Style,
    pub(crate) input_sender: UnboundedSender<T::Input>,
    pub(crate) message_sender: UnboundedSender<T::Message>,
    input_receiver: UnboundedReceiver<T::Input>,
    message_receiver: UnboundedReceiver<T::Message>,
    pub(crate) layout: Layout,
}

#[async_trait]
impl<T> Component for NodeData<T>
where
    T: InteractiveComponent,
{
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        self.component.initialize(context).await
    }
    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>>,
    ) -> KludgineResult<Size> {
        self.component.content_size(context, constraints).await
    }

    async fn layout(
        &mut self,
        context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>> {
        self.component.layout(context).await
    }

    async fn render(&self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        self.component.render(context, layout).await
    }

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        self.component.render_background(context, layout).await
    }

    async fn update(&mut self, context: &mut SceneContext) -> KludgineResult<()> {
        self.component.update(context).await
    }

    async fn process_input(
        &mut self,
        context: &mut Context,
        event: InputEvent,
    ) -> KludgineResult<()> {
        self.component.process_input(context, event).await
    }

    async fn mouse_down(
        &mut self,
        context: &mut Context,
        position: Point,
        button: MouseButton,
    ) -> KludgineResult<EventStatus> {
        self.component.mouse_down(context, position, button).await
    }

    async fn mouse_up(
        &mut self,
        context: &mut Context,
        position: Option<Point>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        self.component.mouse_up(context, position, button).await
    }
}

#[derive(Clone)]
pub struct Node {
    pub(crate) component: KludgineHandle<Box<dyn AnyNode>>,
}

impl Node {
    pub fn new<T: InteractiveComponent + 'static>(
        component: T,
        style: Style,
        hover_style: Style,
        active_style: Style,
        focus_style: Style,
        callback: Option<Callback<T::Output>>,
    ) -> Self {
        let (input_sender, input_receiver) = unbounded_channel();
        let (message_sender, message_receiver) = unbounded_channel();
        Self {
            component: KludgineHandle::new(Box::new(NodeData {
                style,
                hover_style,
                focus_style,
                active_style,
                component,
                input_sender,
                input_receiver,
                message_sender,
                message_receiver,
                callback,
                layout: Default::default(),
            })),
        }
    }

    pub async fn style(&self) -> Style {
        let component = self.component.read().await;
        component.style().clone()
    }

    pub async fn hover_style(&self) -> Style {
        let component = self.component.read().await;
        component.hover_style().clone()
    }

    pub async fn active_style(&self) -> Style {
        let component = self.component.read().await;
        component.active_style().clone()
    }

    pub async fn focus_style(&self) -> Style {
        let component = self.component.read().await;
        component.focus_style().clone()
    }

    pub async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>>,
    ) -> KludgineResult<Size> {
        let component = self.component.read().await;
        component.content_size(context, constraints).await
    }

    pub async fn layout(
        &self,
        context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>> {
        let mut component = self.component.write().await;
        component.layout(context).await
    }

    /// Called once the Window is opened
    pub async fn initialize(&self, context: &mut Context) -> KludgineResult<()> {
        let mut component = self.component.write().await;
        component.initialize(context).await
    }

    pub async fn render(&self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        let component = self.component.read().await;
        component.render(context, layout).await
    }

    pub async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        let component = self.component.read().await;
        component.render_background(context, layout).await
    }

    pub async fn update(&self, context: &mut SceneContext) -> KludgineResult<()> {
        let mut component = self.component.write().await;
        component.update(context).await
    }

    pub async fn process_input(
        &self,
        context: &mut Context,
        event: InputEvent,
    ) -> KludgineResult<()> {
        let mut component = self.component.write().await;
        component.process_input(context, event).await
    }

    pub async fn mouse_down(
        &self,
        context: &mut Context,
        position: Point,
        button: MouseButton,
    ) -> KludgineResult<EventStatus> {
        let mut component = self.component.write().await;
        component.mouse_down(context, position, button).await
    }

    pub async fn mouse_up(
        &self,
        context: &mut Context,
        position: Option<Point>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        let mut component = self.component.write().await;
        component.mouse_up(context, position, button).await
    }

    pub async fn process_pending_events(&self, context: &mut Context) -> KludgineResult<()> {
        let mut component = self.component.write().await;
        component.process_pending_events(context).await
    }

    pub async fn callback<Input: Send + Sync + 'static>(&self, message: Input) {
        let component = self.component.clone();
        Runtime::spawn(async move {
            let component = component.read().await;
            component.send_callback(Box::new(message)).await
        });
    }

    pub(crate) async fn set_layout(&self, layout: Layout) {
        let mut component = self.component.write().await;
        component.set_layout(layout);
    }

    pub async fn last_layout(&self) -> Layout {
        let component = self.component.read().await;
        component.get_layout().clone()
    }
}
