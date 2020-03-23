use crate::{
    runtime::Runtime,
    window::{RuntimeWindow, Window},
};
use std::marker::PhantomData;

#[async_trait]
pub trait Application: Sized + Send + Sync {
    // Async methods
    async fn initialize(&mut self);
    async fn should_exit(&mut self) -> bool {
        RuntimeWindow::count() == 0
    }
}

#[derive(Default)]
pub struct SingleWindowApplication<T> {
    phantom: PhantomData<T>,
}

pub trait WindowCreator<T> {
    fn get_window_builder() -> glutin::window::WindowBuilder {
        glutin::window::WindowBuilder::new().with_title("Kludgine")
    }
}

#[async_trait]
impl<T> Application for SingleWindowApplication<T>
where
    T: Window + Default + WindowCreator<T> + 'static,
{
    async fn initialize(&mut self) {
        Runtime::open_window(T::get_window_builder(), T::default()).await
    }
}
