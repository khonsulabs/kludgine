use super::{
    application::Application,
    window::{RuntimeWindow, Window, WindowBuilder},
    KludgineResult,
};
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use futures::future::Future;
use platforms::target::{OS, TARGET_OS};
use std::time::Duration;
use tokio::runtime::Runtime as TokioRuntime;

pub(crate) enum RuntimeRequest {
    OpenWindow {
        builder: WindowBuilder,
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
    pub(crate) static ref GLOBAL_THREAD_POOL: Mutex<Option<TokioRuntime>> = Mutex::new(None);
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
            async_std::task::sleep(Duration::from_millis(100)).await;
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
        while let Ok(request) = self.request_receiver.try_recv() {
            match request {
                RuntimeRequest::OpenWindow { window, builder } => {
                    let builder: winit::window::WindowBuilder = builder.into();
                    let winit_window = builder.build(&event_loop).unwrap();
                    RuntimeWindow::open(winit_window, window);
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

        *control_flow = winit::event_loop::ControlFlow::Poll;
    }
}

/// Runtime is designed to consume the main thread. For cross-platform compatibility, ensure that you call Runtime::run from thee main thread.
pub struct Runtime {
    request_receiver: Receiver<RuntimeRequest>,
    event_sender: Sender<RuntimeEvent>,
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
            *pool_guard = Some(TokioRuntime::new().expect("Error creating ThreadPool"));
        }
        let app_runtime = ApplicationRuntime { app };
        let (request_receiver, event_sender) = app_runtime.launch();

        Self {
            request_receiver,
            event_sender,
        }
    }

    fn should_run_in_exclusive_mode() -> bool {
        match TARGET_OS {
            OS::Android | OS::iOS => true,
            _ => false,
        }
    }

    pub fn run<T, F>(self, initial_window: WindowBuilder, window: F) -> !
    where
        T: Window + Sized + 'static,
        F: Future<Output = T> + Send + Sync + 'static,
    {
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
        let mut initial_window: winit::window::WindowBuilder = initial_window.into();

        if Self::should_run_in_exclusive_mode() {
            let mut exclusive_mode = None;
            for monitor in event_loop.available_monitors() {
                for mode in monitor.video_modes() {
                    exclusive_mode = Some(mode); // TODO pick the best mode, not the last
                }
            }

            initial_window = initial_window.with_fullscreen(Some(
                winit::window::Fullscreen::Exclusive(exclusive_mode.unwrap()),
            ));
        }
        let window = Runtime::block_on(window);
        let initial_window = initial_window.build(&event_loop).unwrap();
        RuntimeWindow::open(initial_window, Box::new(window));
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
        let pool = GLOBAL_THREAD_POOL.lock().expect("Error getting runtime");
        pool.as_ref().unwrap().spawn(future);
    }

    pub fn block_on<Fut: Future<Output = R> + Send + Sync + 'static, R: Send + Sync + 'static>(
        future: Fut,
    ) -> R {
        let mut pool = GLOBAL_THREAD_POOL.lock().expect("Error getting runtime");
        pool.as_mut().unwrap().block_on(future)
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
