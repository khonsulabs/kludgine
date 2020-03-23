use crate::window::RuntimeWindow;

#[async_trait]
pub trait Application: Sized + Send + Sync {
    // Methods called from the main thread
    fn new() -> Self;

    // Async methods
    async fn initialize(&mut self);
    async fn should_exit(&mut self) -> bool {
        RuntimeWindow::count() == 0
    }
}
