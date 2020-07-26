use crate::{
    math::Size,
    style::Style,
    ui::{
        Callback, Component, Context, InteractiveComponent, Layout, LayoutSolver, SceneContext,
        StyledContext,
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
    fn receive_message(&self, message: Box<dyn Any>);
}

#[async_trait]
pub(crate) trait PendingEventProcessor {
    async fn process_pending_events(&mut self, context: &mut Context) -> KludgineResult<()>;
    async fn send_callback(&mut self, input: Box<dyn Any + Send + Sync>);
}

impl<T: InteractiveComponent + 'static, O: Send + 'static> AnyNode for NodeData<T, O> {
    fn as_any(&self) -> &dyn Any {
        &self.component
    }

    fn style(&self) -> &'_ Style {
        &self.style
    }

    fn receive_message(&self, message: Box<dyn Any>) {
        let message = message.downcast_ref::<T::Message>().unwrap().clone();
        let _ = self.message_sender.send(message);
    }
}

#[async_trait]
impl<T: InteractiveComponent + 'static, O: Send + 'static> PendingEventProcessor
    for NodeData<T, O>
{
    async fn process_pending_events(&mut self, context: &mut Context) -> KludgineResult<()> {
        while let Ok(message) = self.input_receiver.try_recv() {
            self.component.receive_input(context, message).await?
        }
        while let Ok(message) = self.message_receiver.try_recv() {
            self.component.receive_message(context, message).await?
        }
        Ok(())
    }

    async fn send_callback(&mut self, output: Box<dyn Any + Send + Sync>) {
        let output = output.downcast_ref::<T::Output>().unwrap().clone();
        if let Some(callback) = self.callback.as_ref() {
            callback.invoke(output).await;
        }
    }
}

pub struct NodeData<T, O>
where
    T: InteractiveComponent,
{
    component: T,
    callback: Option<Callback<T::Output, O>>,
    pub(crate) style: Style,
    pub(crate) input_sender: UnboundedSender<T::Input>,
    pub(crate) message_sender: UnboundedSender<T::Message>,
    input_receiver: UnboundedReceiver<T::Input>,
    message_receiver: UnboundedReceiver<T::Message>,
}

#[async_trait]
impl<T, O> Component for NodeData<T, O>
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
}

#[derive(Clone)]
pub struct Node {
    pub(crate) component: KludgineHandle<Box<dyn AnyNode>>,
}

impl Node {
    pub fn new<T: InteractiveComponent + 'static, O: Send + 'static>(
        component: T,
        style: Style,
        callback: Option<Callback<T::Output, O>>,
    ) -> Self {
        let (input_sender, input_receiver) = unbounded_channel();
        let (message_sender, message_receiver) = unbounded_channel();
        Self {
            component: KludgineHandle::new(Box::new(NodeData {
                style,
                component,
                input_sender,
                input_receiver,
                message_sender,
                message_receiver,
                callback,
            })),
        }
    }

    pub async fn style(&self) -> Style {
        let component = self.component.read().await;
        component.style().clone()
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

    pub async fn process_pending_events(&self, context: &mut Context) -> KludgineResult<()> {
        let mut component = self.component.write().await;
        component.process_pending_events(context).await
    }

    pub async fn callback<Input: Send + Sync + 'static>(&self, message: Input) {
        let mut component = self.component.write().await;
        component.send_callback(Box::new(message)).await
    }
}
