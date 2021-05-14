use std::marker::PhantomData;

use crate::{
    runtime::Runtime,
    window::{RuntimeWindow, Window, WindowCreator},
};

pub trait Application: Sized + Send + Sync {
    fn initialize(&mut self) {}
    fn should_exit(&mut self) -> bool {
        RuntimeWindow::count() == 0
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
        Runtime::new(app).run(T::get_window_builder(), window)
    }
}
