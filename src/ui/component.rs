use crate::{
    math::{Rect, Size},
    ui::{Context, Placements, SceneContext, StyledContext},
    window::InputEvent,
    KludgineResult,
};
use async_trait::async_trait;

pub struct LayoutConstraints {}

#[async_trait]
pub(crate) trait BaseComponent: Send + Sync {
    /// Called once the Window is opened
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()>;

    async fn update(&mut self, context: &mut SceneContext) -> KludgineResult<()>;

    async fn process_input(
        &mut self,
        context: &mut Context,
        event: InputEvent,
    ) -> KludgineResult<()>;

    async fn layout_within(
        &self,
        context: &mut StyledContext,
        max_size: &Size,
        placements: &Placements,
    ) -> KludgineResult<Size>;

    async fn render(&self, context: &mut StyledContext, location: &Rect) -> KludgineResult<()>;
}

#[async_trait]
pub trait Component: Send + Sync {
    type Message: Send + Sync + std::fmt::Debug;

    /// Called once the Window is opened
    async fn initialize(&mut self, _context: &mut Context) -> KludgineResult<()> {
        Ok(())
    }

    async fn receive_message(
        &mut self,
        _context: &mut Context,
        _message: Self::Message,
    ) -> KludgineResult<()> {
        unimplemented!(
            "Component::receive_message() must be implemented if you're receiving messages"
        )
    }

    async fn layout_within(
        &self,
        _context: &mut StyledContext,
        max_size: &Size,
        _placements: &Placements,
    ) -> KludgineResult<Size> {
        Ok(*max_size)
    }

    async fn render(&self, context: &mut StyledContext, bounds: &Rect) -> KludgineResult<()>;

    async fn update(&mut self, _context: &mut SceneContext) -> KludgineResult<()> {
        Ok(())
    }

    async fn process_input(
        &mut self,
        _context: &mut Context,
        _event: InputEvent,
    ) -> KludgineResult<()> {
        Ok(())
    }
}
