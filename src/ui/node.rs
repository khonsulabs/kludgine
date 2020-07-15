use crate::{
    math::{Rect, Size},
    scene::SceneTarget,
    ui::{BaseComponent, Component, Context},
    window::InputEvent,
    KludgineResult,
};
use async_trait::async_trait;
use std::any::Any;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[async_trait]
pub(crate) trait AnyComponent: BaseComponent {
    fn as_any(&self) -> &dyn Any;
    async fn process_pending_events(&mut self, context: &mut Context) -> KludgineResult<()>;
}

#[async_trait]
impl<T: Component + 'static> AnyComponent for NodeData<T> {
    fn as_any(&self) -> &dyn Any {
        &self.component
    }

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
    async fn content_size(&self, context: &mut Context, max_size: Size) -> KludgineResult<Size> {
        self.component.content_size(context, max_size).await
    }

    async fn render(
        &self,
        context: &mut Context,
        scene: &SceneTarget,
        location: Rect,
    ) -> KludgineResult<()> {
        self.component.render(context, scene, location).await
    }

    async fn update(&mut self, context: &mut Context, scene: &SceneTarget) -> KludgineResult<()> {
        self.component.update(context, scene).await
    }

    async fn process_input(
        &mut self,
        context: &mut Context,
        event: InputEvent,
    ) -> KludgineResult<()> {
        self.component.process_input(context, event).await
    }
}

pub struct Node {
    pub(crate) component: Box<dyn AnyComponent>,
}

impl Node {
    pub fn new<T: Component + 'static>(component: T) -> Self {
        let (sender, receiver) = unbounded_channel();
        Self {
            component: Box::new(NodeData {
                component,
                sender,
                receiver,
            }),
        }
    }
}
