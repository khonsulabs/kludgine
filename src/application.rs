use crate::internal_prelude::*;
use crate::scene2d::Scene2D;

pub enum CloseResponse {
    RemainOpen,
    Close,
}

#[async_trait]
pub trait Application: Sized + Send + Sync {
    // Methods called from the main thread
    fn new() -> Self;

    async fn close_requested(&self) -> CloseResponse {
        CloseResponse::Close
    }

    // Async methods
    async fn initialize(&mut self);
    async fn render_2d(&mut self, _scene: &mut Scene2D) -> KludgineResult<()> {
        Ok(())
    }
}
