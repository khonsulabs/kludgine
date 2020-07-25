use crate::{
    math::Size,
    style::Style,
    ui::{BaseComponent, Component, Context, Layout, LayoutSolver, SceneContext, StyledContext},
    window::InputEvent,
    KludgineHandle, KludgineResult,
};
use async_trait::async_trait;
use std::any::Any;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub(crate) trait AnyNode: PendingEventProcessor + BaseComponent {
    fn as_any(&self) -> &dyn Any;
    fn style(&self) -> &'_ Style;
}

#[async_trait]
pub(crate) trait PendingEventProcessor {
    async fn process_pending_events(&mut self, context: &mut Context) -> KludgineResult<()>;
}

impl<T: Component + 'static> AnyNode for NodeData<T> {
    fn as_any(&self) -> &dyn Any {
        &self.component
    }

    fn style(&self) -> &'_ Style {
        &self.style
    }
}

#[async_trait]
impl<T: Component + 'static> PendingEventProcessor for NodeData<T> {
    async fn process_pending_events(&mut self, context: &mut Context) -> KludgineResult<()> {
        while let Ok(message) = self.receiver.try_recv() {
            self.component.receive_message(context, message).await?
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct NodeData<T>
where
    T: Component,
{
    component: T,
    pub(crate) style: Style,
    pub(crate) sender: UnboundedSender<T::Message>,
    receiver: UnboundedReceiver<T::Message>,
}

#[async_trait]
impl<T> BaseComponent for NodeData<T>
where
    T: Component,
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
    pub fn new<T: Component + 'static>(component: T, style: Style) -> Self {
        let (sender, receiver) = unbounded_channel();
        Self {
            component: KludgineHandle::new(Box::new(NodeData {
                style,
                component,
                sender,
                receiver,
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
}
