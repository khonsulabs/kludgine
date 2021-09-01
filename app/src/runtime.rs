use std::{collections::HashMap, sync::atomic::Ordering};

use kludgine_core::{
    flume,
    winit::{
        self,
        event::Event,
        event_loop::EventLoopProxy,
        window::{Theme, WindowId},
    },
};
use parking_lot::{MappedRwLockReadGuard, Mutex, RwLock, RwLockReadGuard};
use platforms::target::{OS, TARGET_OS};

use crate::{
    application::Application,
    window::{opened_first_window, RuntimeWindow, RuntimeWindowConfig, Window, WindowBuilder},
};

pub enum RuntimeRequest {
    #[cfg(feature = "multiwindow")]
    OpenWindow {
        builder: WindowBuilder,
        window_sender: flume::Sender<RuntimeWindowConfig>,
    },
    WindowClosed,
    Quit,
}

impl RuntimeRequest {
    pub fn send(self) {
        let guard = GLOBAL_RUNTIME_SENDER.lock();
        match *guard {
            Some(ref sender) => {
                let _ = sender.send_event(self);
            }
            None => panic!("Uninitialized runtime"),
        }
    }
}

#[derive(Debug)]
pub enum RuntimeEvent {
    Running,
    WindowClosed,
}

use kludgine_core::lazy_static::lazy_static;

pub trait EventProcessor: Send + Sync {
    fn process_event(
        &mut self,
        event_loop: &winit::event_loop::EventLoopWindowTarget<RuntimeRequest>,
        event: winit::event::Event<'_, RuntimeRequest>,
        control_flow: &mut winit::event_loop::ControlFlow,
    );
}

lazy_static! {
    pub static ref GLOBAL_RUNTIME_SENDER: Mutex<Option<EventLoopProxy<RuntimeRequest>>> =
        Mutex::new(None);
    pub static ref GLOBAL_RUNTIME_RECEIVER: Mutex<Option<flume::Receiver<RuntimeEvent>>> =
        Mutex::new(None);
    pub static ref GLOBAL_EVENT_HANDLER: Mutex<Option<Box<dyn EventProcessor>>> = Mutex::new(None);
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
    fn launch(self) -> flume::Sender<RuntimeEvent> {
        let (event_sender, event_receiver) = flume::unbounded();
        {
            let mut global_receiver = GLOBAL_RUNTIME_RECEIVER.lock();
            assert!(global_receiver.is_none());
            *global_receiver = Some(event_receiver);
        }

        std::thread::Builder::new()
            .name(String::from("kludgine-app"))
            .spawn(move || self.async_main())
            .unwrap();

        event_sender
    }

    fn async_main(mut self)
    where
        App: Application + 'static,
    {
        self.app.initialize();

        self.run();
    }

    fn run(mut self)
    where
        App: Application + 'static,
    {
        let mut running = false;
        let event_receiver = {
            let guard = GLOBAL_RUNTIME_RECEIVER.lock();
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
                RuntimeRequest::Quit.send();
                break;
            }
        }
    }
}

impl EventProcessor for Runtime {
    #[cfg_attr(not(feature = "multiwindow"), allow(unused_variables))] // event_loop is unused if this feature isn't specified
    fn process_event(
        &mut self,
        event_loop: &winit::event_loop::EventLoopWindowTarget<RuntimeRequest>,
        event: winit::event::Event<'_, RuntimeRequest>,
        control_flow: &mut winit::event_loop::ControlFlow,
    ) {
        // while let Ok(request) = self.request_receiver.try_recv() {
        //
        // }
        Self::try_process_window_events(Some(&event));

        match event {
            winit::event::Event::NewEvents(winit::event::StartCause::Init) => {
                self.event_sender
                    .send(RuntimeEvent::Running)
                    .unwrap_or_default();
            }
            winit::event::Event::UserEvent(request) => match request {
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
            },
            _ => {}
        }

        *control_flow = winit::event_loop::ControlFlow::Wait;
    }
}

/// Runtime is designed to consume the main thread. For cross-platform
/// compatibility, ensure that you call [`Runtime::run()`] from thee main
/// thread.
pub struct Runtime {
    event_sender: flume::Sender<RuntimeEvent>,
}

lazy_static! {
    pub static ref WINIT_WINDOWS: RwLock<HashMap<WindowId, winit::window::Window>> =
        RwLock::new(HashMap::new());
}

impl Runtime {
    /// Initializes the managed async runtime.
    pub fn initialize() {
        initialize_async_runtime();
    }

    /// Returns a new runtime for `app`.
    pub fn new<App>(app: App) -> Self
    where
        App: Application + 'static,
    {
        Self::initialize();

        let app_runtime = ApplicationRuntime { app };
        let event_sender = app_runtime.launch();

        Self { event_sender }
    }

    #[cfg(feature = "multiwindow")]
    fn internal_open_window(
        window_sender: &flume::Sender<RuntimeWindowConfig>,
        builder: WindowBuilder,
        event_loop: &winit::event_loop::EventLoopWindowTarget<RuntimeRequest>,
    ) {
        let builder: winit::window::WindowBuilder = builder.into();
        let winit_window = builder.build(event_loop).unwrap();
        window_sender
            .try_send(RuntimeWindowConfig::new(&winit_window))
            .unwrap();

        let mut windows = WINIT_WINDOWS.write();
        windows.insert(winit_window.id(), winit_window);
    }

    const fn should_run_in_exclusive_mode() -> bool {
        matches!(TARGET_OS, OS::Android | OS::iOS)
    }

    /// Executes the runtime's event loop.
    pub fn run<T>(self, initial_window: WindowBuilder, window: T) -> !
    where
        T: Window + Sized + 'static,
    {
        let event_loop = winit::event_loop::EventLoop::<RuntimeRequest>::with_user_event();
        let initial_system_theme = initial_window.initial_system_theme.unwrap_or(Theme::Light);
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

        {
            let mut windows = WINIT_WINDOWS.write();
            windows.insert(initial_window.id(), initial_window);
        }

        {
            let mut global_sender = GLOBAL_RUNTIME_SENDER.lock();
            assert!(global_sender.is_none());
            *global_sender = Some(event_loop.create_proxy());
        }

        // Install the global event handler, and also ensure we aren't trying to
        // initialize two runtimes This is necessary because EventLoop::run requires the
        // function/closure passed to have a `static lifetime for valid reasons. Every
        // approach at using only local variables I could not solve, so we wrap it in a
        // mutex. This abstraction also wraps it in dynamic dispatch, because we can't
        // have a generic-type static variable.
        {
            let mut event_handler = GLOBAL_EVENT_HANDLER.lock();
            assert!(event_handler.is_none());
            *event_handler = Some(Box::new(self));
        }
        event_loop.run(move |event, event_loop, control_flow| {
            let mut event_handler_guard = GLOBAL_EVENT_HANDLER.lock();
            let event_handler = event_handler_guard
                .as_mut()
                .expect("No event handler installed");
            event_handler
                .as_mut()
                .process_event(event_loop, event, control_flow);
        });
    }

    /// Opens a [`Window`]. Requires feature `multiwindow`.
    #[cfg(feature = "multiwindow")]
    pub fn open_window<T>(builder: WindowBuilder, window: T)
    where
        T: Window + Sized,
    {
        let (window_sender, window_receiver) = flume::bounded(1);
        let initial_system_theme = builder.initial_system_theme.unwrap_or(Theme::Light);
        RuntimeRequest::OpenWindow {
            builder,
            window_sender,
        }
        .send();

        RuntimeWindow::open(&window_receiver, initial_system_theme, window);
    }

    pub(crate) fn try_process_window_events(event: Option<&Event<'_, RuntimeRequest>>) -> bool {
        let mut windows = match WINDOWS.try_write() {
            Some(guard) => guard,
            None => return false,
        };

        match event {
            Some(Event::WindowEvent { window_id, event }) => {
                if let Some(window) = windows.get_mut(window_id) {
                    window.process_event(event);
                }
            }
            Some(Event::RedrawRequested(window_id)) => {
                if let Some(window) = windows.get_mut(window_id) {
                    window.request_redraw();
                }
            }
            _ => {}
        }

        {
            for window in windows.values_mut() {
                window.receive_messages();
            }
        }

        if opened_first_window() {
            let mut winit_windows = WINIT_WINDOWS.write();
            windows.retain(|id, w| {
                if w.keep_running.load(Ordering::SeqCst) {
                    true
                } else {
                    winit_windows.remove(id);
                    false
                }
            });
        }

        true
    }

    pub(crate) fn winit_window(
        id: &WindowId,
    ) -> Option<MappedRwLockReadGuard<'static, winit::window::Window>> {
        let windows = WINIT_WINDOWS.read();
        RwLockReadGuard::try_map(windows, |windows| windows.get(id)).ok()
    }
}

fn initialize_async_runtime() {
    #[cfg(feature = "smol-rt")]
    smol::initialize();
    #[cfg(all(feature = "tokio-rt", not(feature = "smol-rt")))]
    tokio::initialize();
}

lazy_static! {
    pub static ref WINDOWS: RwLock<HashMap<WindowId, RuntimeWindow>> = RwLock::new(HashMap::new());
}
