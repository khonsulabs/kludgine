use crate::internal_prelude::*;
use crate::scene2d::Scene2d;

#[async_trait]
pub trait Application: Sized + Send + Sync {
    // Methods called from the main thread
    fn new() -> Self;

    // Async methods
    async fn initialize(&mut self);
}
