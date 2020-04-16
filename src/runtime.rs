use super::{
    application::Application,
    window::{RuntimeWindow, Window},
    KludgineResult,
};
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use futures::{executor::ThreadPool, future::Future};
use std::time::{Duration, Instant};
use winit::window::WindowBuilder;

pub(crate) enum RuntimeRequest {
    // UpdateScene,
    // NewWindow {
    //     notify: oneshot::Sender<KludgineResult<NewWindowResponse>>,
    // },
    OpenWindow {
        builder: winit::window::WindowBuilder,
        window: Box<dyn Window>,
    },
    Quit,
}

impl RuntimeRequest {
    pub async fn send(self) -> KludgineResult<()> {
        let sender: Sender<RuntimeRequest> = {
            let guard = GLOBAL_RUNTIME_SENDER.lock().expect("Error locking mutex");
            match *guard {
                Some(ref sender) => sender.clone(),
                None => panic!("Uninitialized runtime"),
            }
        };
        sender.send(self).unwrap_or_default();
        Ok(())
    }
}

pub(crate) enum RuntimeEvent {
    Running,
}

use lazy_static::lazy_static;
use std::sync::Mutex;

pub trait EventProcessor: Send + Sync {
    fn process_event(
        &mut self,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        event: winit::event::Event<()>,
        control_flow: &mut winit::event_loop::ControlFlow,
    );
}
lazy_static! {
    pub(crate) static ref GLOBAL_RUNTIME_SENDER: Mutex<Option<Sender<RuntimeRequest>>> =
        { Mutex::new(None) };
    pub(crate) static ref GLOBAL_RUNTIME_RECEIVER: Mutex<Option<Receiver<RuntimeEvent>>> =
        { Mutex::new(None) };
    pub(crate) static ref GLOBAL_EVENT_HANDLER: Mutex<Option<Box<dyn EventProcessor>>> =
        Mutex::new(None);
    pub(crate) static ref GLOBAL_THREAD_POOL: Mutex<Option<ThreadPool>> = Mutex::new(None);
}

pub(crate) const FRAME_DURATION: u64 = 6_944_444;

pub struct ApplicationRuntime<App> {
    app: App,
}

impl<App> ApplicationRuntime<App>
where
    App: Application + 'static,
{
    fn launch(self) -> (Receiver<RuntimeRequest>, Sender<RuntimeEvent>) {
        let (event_sender, event_receiver) = unbounded();
        let (request_sender, request_receiver) = unbounded();
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
        self.app.initialize().await;

        self.run()
            .await
            .expect("Error encountered running application loop");
    }

    async fn run(mut self) -> KludgineResult<()>
    where
        App: Application + 'static,
    {
        let mut running = false;
        loop {
            while let Some(event) = {
                let mut guard = GLOBAL_RUNTIME_RECEIVER
                    .lock()
                    .expect("Error locking runtime reciver");
                let event_receiver = guard.as_mut().expect("Receiver was not set");
                match event_receiver.try_recv() {
                    Ok(event) => Some(event),
                    Err(err) => match err {
                        TryRecvError::Empty => None,
                        TryRecvError::Disconnected => return Ok(()),
                    },
                }
            } {
                match event {
                    RuntimeEvent::Running => {
                        running = true;
                    }
                }
            }

            if running && self.app.should_exit().await {
                RuntimeRequest::Quit.send().await?;
                return Ok(());
            }
            async_std::task::sleep(Duration::from_millis(100)).await; // TODO limit
        }
    }
}

impl EventProcessor for Runtime {
    fn process_event(
        &mut self,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        event: winit::event::Event<()>,
        control_flow: &mut winit::event_loop::ControlFlow,
    ) {
        let now = Instant::now();
        let render_frame = now
            .checked_duration_since(self.next_frame_target)
            .unwrap_or_default()
            .as_nanos()
            > (FRAME_DURATION as u128);
        while let Ok(request) = self.request_receiver.try_recv() {
            match request {
                RuntimeRequest::OpenWindow { window, builder } => {
                    RuntimeWindow::open(builder, &event_loop, window);
                }
                RuntimeRequest::Quit => {
                    std::process::exit(0); // TODO There is a bug in winit when destructing https://github.com/rust-windowing/winit/blob/ad7d4939a8be2e0d9436d43d0351e2f7599a4237/src/platform_impl/macos/app_state.rs#L344
                }
            }
        }
        RuntimeWindow::process_events(&event);

        match event {
            winit::event::Event::NewEvents(cause) => match cause {
                winit::event::StartCause::Init => self
                    .event_sender
                    .send(RuntimeEvent::Running)
                    .unwrap_or_default(),
                _ => {}
            },
            _ => {}
        }

        if render_frame {
            self.frame_number += 1;
            RuntimeWindow::render_all();
            self.next_frame_target = self
                .next_frame_target
                .checked_add(Duration::from_nanos(FRAME_DURATION))
                .unwrap_or(self.next_frame_target);
            if self.next_frame_target < now {
                self.next_frame_target = now
                    .checked_add(Duration::from_nanos(FRAME_DURATION))
                    .unwrap_or(now);
            }
            *control_flow = winit::event_loop::ControlFlow::WaitUntil(self.next_frame_target);
        }
    }
}

/// Runtime is designed to consume the main thread. For cross-platform compatibility, ensure that you call Runtime::run from thee main thread.
pub struct Runtime {
    request_receiver: Receiver<RuntimeRequest>,
    event_sender: Sender<RuntimeEvent>,
    next_frame_target: Instant,
    frame_number: u64,
}

impl Runtime {
    pub fn new<App>(app: App) -> Self
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
        let app_runtime = ApplicationRuntime { app };
        let (request_receiver, event_sender) = app_runtime.launch();

        Self {
            request_receiver,
            event_sender,
            next_frame_target: Instant::now(),
            frame_number: 0,
        }
    }

    pub fn run(self) -> ! {
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

        let event_loop = winit::event_loop::EventLoop::new();
        event_loop.run(move |event, event_loop, control_flow| {
            let mut event_handler_guard = GLOBAL_EVENT_HANDLER
                .lock()
                .expect("Error locking main event handler");
            let event_handler = event_handler_guard
                .as_mut()
                .expect("No event handler installed");
            event_handler
                .as_mut()
                .process_event(&event_loop, event, control_flow);
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

    pub async fn open_window<T>(builder: WindowBuilder, window: T)
    where
        T: Window + Sized,
    {
        RuntimeRequest::OpenWindow {
            builder,
            window: Box::new(window),
        }
        .send()
        .await
        .unwrap_or_default();
    }
}
