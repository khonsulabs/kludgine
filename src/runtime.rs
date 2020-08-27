use super::{
    application::Application,
    window::{RuntimeWindow, Window, WindowBuilder},
    KludgineResult,
};
use crossbeam::{
    channel::{unbounded, Receiver, Sender, TryRecvError},
    sync::ShardedLock,
};
use futures::future::Future;
use platforms::target::{OS, TARGET_OS};
use std::time::Duration;

pub(crate) enum RuntimeRequest {
    OpenWindow {
        builder: WindowBuilder,
        window_sender: async_channel::Sender<winit::window::Window>,
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
        Mutex::new(None);
    pub(crate) static ref GLOBAL_RUNTIME_RECEIVER: Mutex<Option<Receiver<RuntimeEvent>>> =
        Mutex::new(None);
    pub(crate) static ref GLOBAL_EVENT_HANDLER: Mutex<Option<Box<dyn EventProcessor>>> =
        Mutex::new(None);
    pub(crate) static ref GLOBAL_THREAD_POOL: ShardedLock<Option<smol::Executor>> =
        ShardedLock::new(None);
}

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
        Runtime::spawn(self.async_main()).detach();

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
                RuntimeRequest::OpenWindow {
                    window_sender,
                    builder,
                } => {
                    self.internal_open_window(window_sender, builder, event_loop);
                }
                RuntimeRequest::Quit => {
                    std::process::exit(0); // TODO There is a bug in winit when destructing https://github.com/rust-windowing/winit/blob/ad7d4939a8be2e0d9436d43d0351e2f7599a4237/src/platform_impl/macos/app_state.rs#L344
                }
            }
        }
        RuntimeWindow::process_events(&event);

        if let winit::event::Event::NewEvents(winit::event::StartCause::Init) = event {
            self.event_sender
                .send(RuntimeEvent::Running)
                .unwrap_or_default();
        }

        *control_flow = winit::event_loop::ControlFlow::Wait;
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
            {
                let mut pool_guard = GLOBAL_THREAD_POOL
                    .write()
                    .expect("Error locking global thread pool");
                assert!(pool_guard.is_none());
                let executor = smol::Executor::new();
                *pool_guard = Some(executor);
            }

            // Launch a thread pool
            std::thread::spawn(|| {
                let (signal, shutdown) = async_channel::unbounded::<()>();

                easy_parallel::Parallel::new()
                    // Run four executor threads.
                    .each(0..4, |_| {
                        futures::executor::block_on(async {
                            let guard = GLOBAL_THREAD_POOL.read().unwrap();
                            let executor = guard.as_ref().unwrap();
                            executor.run(shutdown.recv()).await
                        })
                    })
                    // Run the main future on the current thread.
                    .finish(|| {});

                signal.close();
            });
        }
        let app_runtime = ApplicationRuntime { app };
        let (request_receiver, event_sender) = app_runtime.launch();

        Self {
            request_receiver,
            event_sender,
        }
    }

    fn internal_open_window(
        &self,
        window_sender: async_channel::Sender<winit::window::Window>,
        builder: WindowBuilder,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
    ) {
        let builder: winit::window::WindowBuilder = builder.into();
        let winit_window = builder.build(&event_loop).unwrap();
        window_sender
            .try_send(winit_window)
            .expect("Couldn't send winit window");
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
        let (window_sender, window_receiver) = async_channel::bounded(1);
        window_sender.try_send(initial_window).unwrap();
        Runtime::block_on(RuntimeWindow::open(window_receiver, window));
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

    pub fn spawn<Fut: Future<Output = T> + Send + 'static, T: Send + 'static>(
        future: Fut,
    ) -> smol::Task<T> {
        let guard = GLOBAL_THREAD_POOL.read().expect("Error getting runtime");
        let executor = guard.as_ref().unwrap();
        executor.spawn(future)
    }

    pub fn block_on<'a, Fut: Future<Output = R> + Send + 'a, R: Send + Sync + 'a>(
        future: Fut,
    ) -> R {
        futures::executor::block_on(future)
    }

    pub async fn open_window<T>(builder: WindowBuilder, window: T)
    where
        T: Window + Sized,
    {
        let (window_sender, window_receiver) = async_channel::bounded(1);
        RuntimeRequest::OpenWindow {
            builder,
            window_sender,
        }
        .send()
        .await
        .unwrap_or_default();

        RuntimeWindow::open(window_receiver, window).await;
    }
}
