use crate::{ui::Context, KludgineResult};
use async_trait::async_trait;

pub struct LayoutConstraints {}

#[async_trait]
pub(crate) trait BaseComponent: Send + Sync + std::fmt::Debug {
    async fn layout(&mut self, context: &mut Context) -> KludgineResult<LayoutConstraints>;

    async fn render(&self, context: &mut Context) -> KludgineResult<()>;

    async fn update(&mut self, context: &mut Context) -> KludgineResult<()>;
}

#[async_trait]
pub trait Component: Send + Sync + std::fmt::Debug {
    type Message;
    async fn layout(&mut self, context: &mut Context) -> KludgineResult<LayoutConstraints>;

    async fn render(&self, context: &mut Context) -> KludgineResult<()>;

    async fn update(&mut self, context: &mut Context) -> KludgineResult<()> {
        Ok(())
    }
}

#[async_trait]
impl<T> BaseComponent for T
where
    T: Component,
{
    async fn layout(&mut self, context: &mut Context) -> KludgineResult<LayoutConstraints> {
        self.layout(context).await
    }

    async fn render(&self, context: &mut Context) -> KludgineResult<()> {
        self.render(context).await
    }

    async fn update(&mut self, context: &mut Context) -> KludgineResult<()> {
        self.update(context).await
    }
}
