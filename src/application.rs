use crate::{
    runtime::Runtime,
    window::{RuntimeWindow, Window, WindowCreator},
};
use async_trait::async_trait;
use futures::Future;
use std::marker::PhantomData;

#[async_trait]
pub trait Application: Sized + Send + Sync {
    // Async methods
    async fn initialize(&mut self) {}
    async fn should_exit(&mut self) -> bool {
        RuntimeWindow::count().await == 0
    }
}

pub struct SingleWindowApplication<T> {
    phantom: PhantomData<T>,
}

impl<T> Application for SingleWindowApplication<T> where T: Window + WindowCreator + 'static {}

impl<T> SingleWindowApplication<T>
where
    T: Window + WindowCreator + 'static,
{
    pub fn run(window: T) -> ! {
        let app = Self {
            phantom: PhantomData::default(),
        };
        Runtime::new(app).run(T::get_window_builder(), async move { window })
    }

    pub fn run_with<C, F>(window_func: C) -> !
    where
        C: FnOnce() -> F,
        F: Future<Output = T> + Send + Sync + 'static,
    {
        let app = Self {
            phantom: PhantomData::default(),
        };
        Runtime::new(app).run(T::get_window_builder(), window_func())
    }
}
