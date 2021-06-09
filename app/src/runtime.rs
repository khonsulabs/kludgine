use std::{collections::HashMap, sync::atomic::Ordering};

use crossbeam::{
    channel::{unbounded, Receiver, Sender},
    sync::ShardedLock,
};
use kludgine_core::{
    flume,
    winit::{
        self,
        event::Event,
        window::{Theme, WindowId},
    },
};
use platforms::target::{OS, TARGET_OS};

use crate::{
    application::Application,
    window::{opened_first_window, RuntimeWindow, RuntimeWindowConfig, Window, WindowBuilder},
};

pub(crate) enum RuntimeRequest {
    #[cfg(feature = "multiwindow")]
    OpenWindow {
        builder: WindowBuilder,
        window_sender: flume::Sender<RuntimeWindowConfig>,
    },
    WindowClosed,
    Quit,
}

impl RuntimeRequest {
    pub fn send(self) -> crate::Result<()> {
        let sender: Sender<Self> = {
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

#[derive(Debug)]
pub(crate) enum RuntimeEvent {
    Running,
    WindowClosed,
}

use std::sync::Mutex;

use kludgine_core::lazy_static::lazy_static;

pub trait EventProcessor: Send + Sync {
    fn process_event(
        &mut self,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        event: winit::event::Event<'_, ()>,
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
}

#[cfg(feature = "smol-rt")]
mod smol;

#[cfg(feature = "tokio-rt")]
mod tokio;

#[cfg(target_arch = "wasm32")]
mod web_sys;

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

        std::thread::Builder::new()
            .name(String::from("kludgine-app"))
            .spawn(move || self.async_main())
            .unwrap();

        (request_receiver, event_sender)
    }

    fn async_main(mut self)
    where
        App: Application + 'static,
    {
        self.app.initialize();

        self.run()
            .expect("Error encountered running application loop");
    }

    fn run(mut self) -> crate::Result<()>
    where
        App: Application + 'static,
    {
        let mut running = false;
        let event_receiver = {
            let guard = GLOBAL_RUNTIME_RECEIVER
                .lock()
                .expect("Error locking runtime reciver");
            guard.as_ref().expect("Receiver was not set").clone()
        };
        while let Some(event) = event_receiver.recv().ok() {
            match event {
                RuntimeEvent::Running => {
                    running = true;
                }
                RuntimeEvent::WindowClosed => {}
            }

            if running && self.app.should_exit() {
                RuntimeRequest::Quit.send()?;
                break;
            }
        }

        Ok(())
    }
}

impl EventProcessor for Runtime {
    #[cfg_attr(not(feature = "multiwindow"), allow(unused_variables))] // event_loop is unused if this feature isn't specified
    fn process_event(
        &mut self,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        event: winit::event::Event<'_, ()>,
        control_flow: &mut winit::event_loop::ControlFlow,
    ) {
        while let Ok(request) = self.request_receiver.try_recv() {
            match request {
                #[cfg(feature = "multiwindow")]
                RuntimeRequest::OpenWindow {
                    window_sender,
                    builder,
                } => {
                    Self::internal_open_window(&window_sender, builder, event_loop);
                }
                RuntimeRequest::Quit => {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                    return;
                }
                RuntimeRequest::WindowClosed => {
                    self.event_sender
                        .send(RuntimeEvent::WindowClosed)
                        .unwrap_or_default();
                }
            }
        }
        Self::process_window_events(&event);

        if let winit::event::Event::NewEvents(winit::event::StartCause::Init) = event {
            self.event_sender
                .send(RuntimeEvent::Running)
                .unwrap_or_default();
        }

        *control_flow = winit::event_loop::ControlFlow::Wait;
    }
}

/// Runtime is designed to consume the main thread. For cross-platform
/// compatibility, ensure that you call [`Runtime::run()`] from thee main
/// thread.
pub struct Runtime {
    request_receiver: Receiver<RuntimeRequest>,
    event_sender: Sender<RuntimeEvent>,
}

#[cfg(feature = "multiwindow")]
lazy_static! {
    pub(crate) static ref WINIT_WINDOWS: ShardedLock<HashMap<WindowId, winit::window::Window>> =
        ShardedLock::new(HashMap::new());
}

impl Runtime {
    pub fn initialize() {
        initialize_async_runtime();
    }

    pub fn new<App>(app: App) -> Self
    where
        App: Application + 'static,
    {
        Self::initialize();

        let app_runtime = ApplicationRuntime { app };
        let (request_receiver, event_sender) = app_runtime.launch();

        Self {
            request_receiver,
            event_sender,
        }
    }

    #[cfg(feature = "multiwindow")]
    fn internal_open_window(
        window_sender: &flume::Sender<RuntimeWindowConfig>,
        builder: WindowBuilder,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
    ) {
        let builder: winit::window::WindowBuilder = builder.into();
        let winit_window = builder.build(event_loop).unwrap();
        window_sender
            .try_send(RuntimeWindowConfig::new(&winit_window))
            .unwrap();

        let mut windows = WINIT_WINDOWS.write().unwrap();
        windows.insert(winit_window.id(), winit_window);
    }

    const fn should_run_in_exclusive_mode() -> bool {
        matches!(TARGET_OS, OS::Android | OS::iOS)
    }

    pub fn run<T>(self, initial_window: WindowBuilder, window: T) -> !
    where
        T: Window + Sized + 'static,
    {
        let event_loop = winit::event_loop::EventLoop::new();
        let initial_system_theme = initial_window
            .initial_system_theme
            .clone()
            .unwrap_or(Theme::Light);
        let mut initial_window: winit::window::WindowBuilder = initial_window.into();

        if Self::should_run_in_exclusive_mode() {
            let mut exclusive_mode = None;
            for monitor in event_loop.available_monitors() {
                for mode in monitor.video_modes() {
                    exclusive_mode = Some(mode); // TODO pick the best mode, not
                                                 // the last
                }
            }

            initial_window = initial_window.with_fullscreen(Some(
                winit::window::Fullscreen::Exclusive(exclusive_mode.unwrap()),
            ));
        }
        let initial_window = initial_window.build(&event_loop).unwrap();
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            // On wasm, append the canvas to the document body
            ::web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| doc.body())
                .and_then(|body| {
                    body.append_child(&::web_sys::Element::from(initial_window.canvas()))
                        .ok()
                })
                .expect("couldn't append canvas to document body");
        }
        let (window_sender, window_receiver) = flume::bounded(1);
        window_sender
            .send(RuntimeWindowConfig::new(&initial_window))
            .unwrap();

        RuntimeWindow::open(&window_receiver, initial_system_theme, window);

        #[cfg(feature = "multiwindow")]
        {
            let mut windows = WINIT_WINDOWS.write().unwrap();
            windows.insert(initial_window.id(), initial_window);
        }

        // Install the global event handler, and also ensure we aren't trying to
        // initialize two runtimes This is necessary because EventLoop::run requires the
        // function/closure passed to have a `static lifetime for valid reasons. Every
        // approach at using only local variables I could not solve, so we wrap it in a
        // mutex. This abstraction also wraps it in dynamic dispatch, because we can't
        // have a generic-type static variable.
        {
            let mut event_handler = GLOBAL_EVENT_HANDLER
                .lock()
                .expect("Error locking global event handler");
            assert!(event_handler.is_none());
            *event_handler = Some(Box::new(self));
        }
        event_loop.run(move |event, event_loop, control_flow| {
            let mut event_handler_guard = GLOBAL_EVENT_HANDLER
                .lock()
                .expect("Error locking main event handler");
            let event_handler = event_handler_guard
                .as_mut()
                .expect("No event handler installed");
            event_handler
                .as_mut()
                .process_event(event_loop, event, control_flow);
        });
    }

    #[cfg(feature = "multiwindow")]
    pub fn open_window<T>(builder: WindowBuilder, window: T)
    where
        T: Window + Sized,
    {
        let (window_sender, window_receiver) = flume::bounded(1);
        let initial_system_theme = builder.initial_system_theme.clone().unwrap_or(Theme::Light);
        RuntimeRequest::OpenWindow {
            builder,
            window_sender,
        }
        .send()
        .unwrap_or_default();

        RuntimeWindow::open(&window_receiver, initial_system_theme, window);
    }

    fn process_window_events(event: &Event<'_, ()>) {
        let mut windows = WINDOWS.write().unwrap();

        if let Event::WindowEvent { window_id, event } = event {
            if let Some(window) = windows.get_mut(window_id) {
                window.process_event(event);
            }
        } else if let Event::RedrawRequested(window_id) = event {
            if let Some(window) = windows.get_mut(window_id) {
                window.request_redraw();
            }
        }

        {
            for window in windows.values_mut() {
                window.receive_messages();
            }
        }

        if opened_first_window() {
            #[cfg(not(feature = "multiwindow"))]
            {
                let old_count = windows.len();
                windows.retain(|_, w| w.keep_running.load());
            }

            #[cfg(feature = "multiwindow")]
            {
                let mut winit_windows = WINIT_WINDOWS.write().unwrap();
                windows.retain(|id, w| {
                    if w.keep_running.load(Ordering::SeqCst) {
                        true
                    } else {
                        winit_windows.remove(id);
                        false
                    }
                });
            };
        }
    }
}

fn initialize_async_runtime() {
    #[cfg(feature = "smol-rt")]
    smol::initialize();
    #[cfg(all(feature = "tokio-rt", not(feature = "smol-rt")))]
    tokio::initialize();
}

lazy_static! {
    pub(crate) static ref WINDOWS: ShardedLock<HashMap<WindowId, RuntimeWindow>> =
        ShardedLock::new(HashMap::new());
}
