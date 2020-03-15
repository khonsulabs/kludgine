use crate::application::{Application, CloseResponse};
use crate::internal_prelude::*;
use futures::executor::ThreadPool;
use futures::prelude::*;
use glutin::window::WindowId;
use lazy_static::lazy_static;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

mod request;
mod threading;
use request::*;
use threading::*;

pub struct ApplicationRuntime<App> {
    app: App,
    pool: ThreadPool,
}
type ApplicationRuntimeHandle<App> = Arc<Mutex<ApplicationRuntime<App>>>;
pub type RuntimeHandle<App> = Arc<Mutex<Runtime<App>>>;

pub trait RuntimeHandleMethods {
    fn run(self);
}

/// Runtime is designed to consume the main thread. For cross-platform compatibility, ensure that you call Runtime::run from thee main thread.
pub struct Runtime<App>
where
    App: Application,
{
    app_runtime: ApplicationRuntimeHandle<App>,
    receiver: Mutex<mpsc::UnboundedReceiver<RuntimeRequest>>,
}

impl<App> Runtime<App>
where
    App: Application + 'static,
{
    pub fn new() -> RuntimeHandle<App> {
        let pool = ThreadPool::new().expect("Error creating ThreadPool");
        let app = App::new();
        let app_runtime = Arc::new(Mutex::new(ApplicationRuntime { app, pool }));
        let receiver = app_runtime.launch();

        Arc::new(Mutex::new(Self {
            app_runtime,
            receiver: Mutex::new(receiver),
        }))
    }

    /// Checks that the handle i
    fn process_event(
        &mut self,
        event: glutin::event::Event<()>,
        control_flow: &mut glutin::event_loop::ControlFlow,
        display: &glium::Display,
    ) {
        if self.app_runtime.should_quit() {
            *control_flow = glutin::event_loop::ControlFlow::Exit;
            return;
        }
        while let Some(request) = self
            .receiver
            .lock()
            .expect("Error locking receiver")
            .try_next()
            .unwrap_or_default()
        {
            match request {}
        }

        let now = Instant::now();
        let next_frame_time = now + Duration::from_nanos(16_666_667);
        let render_frame = match event {
            glutin::event::Event::WindowEvent { event, .. } => match event {
                glutin::event::WindowEvent::CloseRequested => {
                    if let CloseResponse::Close = self.app_runtime.close_requested() {
                        *control_flow = glutin::event_loop::ControlFlow::Exit;
                    }
                    false
                }
                _ => false,
            },
            glutin::event::Event::NewEvents(cause) => match cause {
                glutin::event::StartCause::ResumeTimeReached { .. } => true,
                glutin::event::StartCause::Init => true,
                _ => false,
            },
            _ => false,
        };

        if render_frame {
            // Dome some stuff
            *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);
        }
    }
}

impl<App> RuntimeHandleMethods for RuntimeHandle<App>
where
    App: Application + 'static,
{
    fn run(self) {
        // Install the global event handler, and also ensure we aren't trying to initialize two runtimes
        // This is necessary because EventLoop::run requires the function/closure passed to have a `static
        // lifetime for valid reasons. Every approach at using only local variables I could not solve, so
        // we wrap it in a mutex. This abstraction also wraps it in dynamic dispatch, because we can't have
        // a generic-type static variable.
        {
            let mut event_handler = GLOBAL_EVENT_HANDLER
                .lock()
                .expect("Error locking global event handler");
            assert!(event_handler.is_none());
            *event_handler = Some(Box::new(self));
        }

        let event_loop = glutin::event_loop::EventLoop::new();
        let wb = glutin::window::WindowBuilder::new()
            .with_title("Cosmic Verge") // TODO Remove hardcoded name
            .with_inner_size(glutin::dpi::LogicalSize::new(1920.0, 1080.0)); // TODO remove hardcoded size
        let cb = glutin::ContextBuilder::new();

        let display = glium::Display::new(wb, cb, &event_loop).unwrap();
        event_loop.run(move |event, _, control_flow| {
            let mut event_handler_guard = GLOBAL_EVENT_HANDLER
                .lock()
                .expect("Error locking main event handler");
            let event_handler = event_handler_guard
                .as_mut()
                .expect("No event handler installed");
            event_handler
                .as_mut()
                .process_event(event, control_flow, &display);
        });
    }
}

async fn application_main<App>(runtime: ApplicationRuntimeHandle<App>)
where
    App: Application + 'static,
{
    {
        let mut guard = runtime.lock().expect("Error locking runtime");
        guard.app.initialize();
    }
}
