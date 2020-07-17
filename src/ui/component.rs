use crate::{
    math::{Rect, Size},
    scene::SceneTarget,
    style::EffectiveStyle,
    ui::{Context, Placements},
    window::InputEvent,
    KludgineResult,
};
use async_trait::async_trait;

pub struct LayoutConstraints {}

#[async_trait]
pub(crate) trait BaseComponent: Send + Sync {
    async fn layout_within(
        &self,
        context: &mut Context,
        max_size: Size,
        effective_style: &EffectiveStyle,
        placements: &Placements,
    ) -> KludgineResult<Size>;

    /// Called once the Window is opened
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()>;

    async fn render(
        &self,
        context: &mut Context,
        scene: &SceneTarget,
        location: Rect,
        effective_style: &EffectiveStyle,
    ) -> KludgineResult<()>;

    async fn update(&mut self, context: &mut Context, scene: &SceneTarget) -> KludgineResult<()>;

    async fn process_input(
        &mut self,
        context: &mut Context,
        event: InputEvent,
    ) -> KludgineResult<()>;
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
        _context: &mut Context,
        max_size: Size,
        _effective_style: &EffectiveStyle,
        _placements: &Placements,
    ) -> KludgineResult<Size> {
        Ok(max_size)
    }

    async fn render(
        &self,
        context: &mut Context,
        scene: &SceneTarget,
        location: Rect,
        effective_style: &EffectiveStyle,
    ) -> KludgineResult<()>;

    async fn update(&mut self, _context: &mut Context, _scene: &SceneTarget) -> KludgineResult<()> {
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
