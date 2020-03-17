use crate::application::{Application, CloseResponse};
use crate::internal_prelude::*;
use crate::scene2d::Scene2D;
use futures::{executor::ThreadPool, future::Future};
use std::{
    borrow::Borrow,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

mod request;
mod threading;
use request::*;
use threading::*;

pub struct ApplicationRuntime<App> {
    app: App,
}

impl<App> ApplicationRuntime<App>
where
    App: Application + 'static,
{
    fn launch(
        self,
    ) -> (
        mpsc::UnboundedReceiver<RuntimeRequest>,
        mpsc::UnboundedSender<RuntimeEvent>,
    ) {
        let (event_sender, event_receiver) = mpsc::unbounded();
        let (request_sender, request_receiver) = mpsc::unbounded();
        {
            let mut global_sender = GLOBAL_RUNTIME_SENDER
                .lock()
                .expect("Error locking global sender");
            assert!(global_sender.is_none());
            *global_sender = Some(request_sender);
            let mut global_receiver = GLOBAL_RUNTIME_RECEIVER
                .lock()
                .expect("Error locking global receiver");
            assert!(global_receiver.is_none());
            *global_receiver = Some(event_receiver);
        }
        Runtime::spawn(self.async_main());
        (request_receiver, event_sender)
    }

    async fn async_main(mut self)
    where
        App: Application + 'static,
    {
        self.app.initialize();

        self.run()
            .await
            .expect("Error encountered running application loop");
    }

    async fn run(mut self) -> KludgineResult<()>
    where
        App: Application + 'static,
    {
        let mut scene2d = Scene2D::new();
        loop {
            {
                while let Some(event) = {
                    let mut guard = GLOBAL_RUNTIME_RECEIVER
                        .lock()
                        .expect("Error locking runtime reciver");
                    let event_receiver = guard.as_mut().expect("Receiver was not set");
                    event_receiver.try_next().unwrap_or_default()
                } {
                    match event {
                        RuntimeEvent::CloseRequested => {
                            if let CloseResponse::Close = self.app.close_requested().await {
                                return RuntimeRequest::Quit.send().await;
                            }
                        }
                    }
                }
            }
            self.app.render_2d(&mut scene2d).await?;
        }
        Ok(())
    }
}

impl EventProcessor for Runtime {
    /// Checks that the handle i
    fn process_event(
        &mut self,
        event: glutin::event::Event<()>,
        control_flow: &mut glutin::event_loop::ControlFlow,
        display: &glium::Display,
    ) {
        while let Some(request) = self.request_receiver.try_next().unwrap_or_default() {
            match request {
                RuntimeRequest::Quit => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                    return;
                }
            }
        }

        let now = Instant::now();
        let next_frame_time = now + Duration::from_nanos(16_666_667);
        let render_frame = match event {
            glutin::event::Event::WindowEvent { event, .. } => match event {
                glutin::event::WindowEvent::CloseRequested => {
                    block_on(self.event_sender.send(RuntimeEvent::CloseRequested))
                        .unwrap_or_default();
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

/// Runtime is designed to consume the main thread. For cross-platform compatibility, ensure that you call Runtime::run from thee main thread.
pub struct Runtime {
    request_receiver: mpsc::UnboundedReceiver<RuntimeRequest>,
    event_sender: mpsc::UnboundedSender<RuntimeEvent>,
}

impl Runtime {
    pub fn new<App>() -> Self
    where
        App: Application + 'static,
    {
        {
            let mut pool_guard = GLOBAL_THREAD_POOL
                .lock()
                .expect("Error locking global thread pool");
            assert!(pool_guard.is_none());
            *pool_guard = Some(ThreadPool::new().expect("Error creating ThreadPool"));
        }
        let app = App::new();
        let app_runtime = ApplicationRuntime { app };
        let (request_receiver, event_sender) = app_runtime.launch();

        Self {
            request_receiver,
            event_sender,
        }
    }

    pub fn run(self) {
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

    pub fn spawn<Fut: Future<Output = ()> + Send + 'static>(future: Fut) {
        let pool = {
            let guard = GLOBAL_THREAD_POOL
                .lock()
                .expect("Error locking thread pool");
            guard.as_ref().expect("No thread pool created yet").clone()
        };
        pool.spawn_ok(future);
    }
}
