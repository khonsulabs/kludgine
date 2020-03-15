use super::{request::RuntimeRequest, ApplicationRuntimeHandle, CloseResponse, RuntimeHandle};
use crate::application::Application;
use crate::internal_prelude::*;
use lazy_static::lazy_static;
use std::sync::Mutex;

pub trait EventProcessor: Send + Sync {
    fn process_event(
        &mut self,
        event: glutin::event::Event<()>,
        control_flow: &mut glutin::event_loop::ControlFlow,
        display: &glium::Display,
    );
}
lazy_static! {
    pub(crate) static ref GLOBAL_RUNTIME_SENDER: Mutex<Option<mpsc::UnboundedSender<RuntimeRequest>>> =
        { Mutex::new(None) };
    pub(crate) static ref GLOBAL_EVENT_HANDLER: Mutex<Option<Box<dyn EventProcessor>>> =
        Mutex::new(None);
}

impl<App> EventProcessor for RuntimeHandle<App>
where
    App: Application + 'static,
{
    fn process_event(
        &mut self,
        event: glutin::event::Event<()>,
        control_flow: &mut glutin::event_loop::ControlFlow,
        display: &glium::Display,
    ) {
        let mut guard = self.lock().expect("Error locking runtime");
        guard.process_event(event, control_flow, display);
    }
}

pub(crate) trait ThreadSafeApplicationRuntime<App> {
    fn launch(&self) -> mpsc::UnboundedReceiver<RuntimeRequest>;
    fn should_quit(&self) -> bool;
    fn close_requested(&self) -> CloseResponse;
}

impl<App> ThreadSafeApplicationRuntime<App> for ApplicationRuntimeHandle<App>
where
    App: Application + 'static,
{
    fn launch(&self) -> mpsc::UnboundedReceiver<RuntimeRequest> {
        let thread_runtime = self.clone();
        let (sender, receiver) = mpsc::unbounded();
        let mut global_sender = GLOBAL_RUNTIME_SENDER
            .lock()
            .expect("Error locking global sender");
        assert!(global_sender.is_none());
        *global_sender = Some(sender);

        let pool = {
            let guard = self.lock().expect("Error locking runtime");
            guard.pool.clone()
        };
        pool.spawn_ok(super::application_main(thread_runtime));
        receiver
    }

    fn should_quit(&self) -> bool {
        let guard = self.lock().expect("Error locking runtime");
        guard.app.should_quit()
    }

    fn close_requested(&self) -> CloseResponse {
        let guard = self.lock().expect("Error locking runtime");
        guard.app.close_requested()
    }
}
