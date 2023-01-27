use std::marker::PhantomData;

use crate::runtime::Runtime;
use crate::window::{RuntimeWindow, Window, WindowCreator};

/// A trait that describes the application's behavior.
pub trait Application: Sized + Send + Sync {
    /// Executed upon application launch.
    fn initialize(&mut self) {}

    /// Return true if the app should exit. Default implementation returns true
    /// once [`Application::open_window_count()`] returns zero.
    fn should_exit(&mut self) -> bool {
        Self::open_window_count() == 0
    }

    /// Returns the number of open windows.
    #[must_use]
    fn open_window_count() -> usize {
        RuntimeWindow::count()
    }
}

/// An [`Application`] implementation that begins with a single window.
///
/// If feature `multiwindow` is enabled, multiple windows can still be opened.
/// This structure just provides a way to run an app without explicitly
/// implementing [`Application`] on one of your types.
pub struct SingleWindowApplication<T> {
    phantom: PhantomData<T>,
}

impl<T> Application for SingleWindowApplication<T> where T: Window + WindowCreator + 'static {}

impl<T> SingleWindowApplication<T>
where
    T: Window + WindowCreator + 'static,
{
    /// Runs the app. Does not return.
    pub fn run(window: T) -> ! {
        let app = Self {
            phantom: PhantomData::default(),
        };
        Runtime::new(app).run(window.get_window_builder(), window)
    }
}
