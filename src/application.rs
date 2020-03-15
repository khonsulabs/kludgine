use crate::internal_prelude::*;
use crate::runtime::Runtime;

pub enum CloseResponse {
    RemainOpen,
    Close,
}

#[async_trait]
pub trait Application: Sized + Send + Sync {
    // Methods called from the main thread
    fn new() -> Self;
    fn should_quit(&self) -> bool;

    fn close_requested(&self) -> CloseResponse {
        CloseResponse::Close
    }

    // Async methods
    async fn initialize(&mut self);
}
