use super::{
    application::WindowCreator,
    event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, TouchPhase, VirtualKeyCode},
    frame::Frame,
    math::{Point, Size},
    runtime::{Runtime, FRAME_DURATION},
    scene::{Scene, SceneTarget},
    ui::{global_arena, InteractiveComponent, NodeData, NodeDataWindowExt, UserInterface},
    KludgineError, KludgineHandle, KludgineResult,
};
use async_trait::async_trait;

use crossbeam::{
    atomic::AtomicCell,
    channel::{unbounded, Receiver, Sender, TryRecvError},
    sync::ShardedLock,
};
use lazy_static::lazy_static;
use rgx::core::*;

use futures::executor::block_on;
use std::{collections::HashMap, sync::Arc, time::Duration};
use winit::{
    event::{Event as WinitEvent, WindowEvent as WinitWindowEvent},
    window::{Window as WinitWindow, WindowBuilder as WinitWindowBuilder, WindowId},
};

mod renderer;
use renderer::{FrameRenderer, FrameSynchronizer};

pub use winit::window::Icon;

/// How to react to a request to close a window
pub enum CloseResponse {
    /// Window should remain open
    RemainOpen,
    /// Window should close
    Close,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum EventStatus {
    Ignored,
    Processed,
}

impl Default for EventStatus {
    fn default() -> Self {
        EventStatus::Ignored
    }
}

impl EventStatus {
    pub fn update_with(&mut self, other: Self) {
        *self = if self == &EventStatus::Processed || other == EventStatus::Processed {
            EventStatus::Processed
        } else {
            EventStatus::Ignored
        };
    }
}

/// An Event from a device
#[derive(Copy, Clone)]
pub struct InputEvent {
    /// The device that triggered this event
    pub device_id: DeviceId,
    /// The event that was triggered
    pub event: Event,
}

/// An input Event
#[derive(Copy, Clone)]
pub enum Event {
    /// A keyboard event
    Keyboard {
        key: Option<VirtualKeyCode>,
        state: ElementState,
    },
    /// A mouse button event
    MouseButton {
        button: MouseButton,
        state: ElementState,
    },
    /// Mouse cursor event
    MouseMoved { position: Option<Point> },
    /// Mouse wheel event
    MouseWheel {
        delta: MouseScrollDelta,
        touch_phase: TouchPhase,
    },
}

/// Trait to implement a Window
#[async_trait]
pub trait Window: InteractiveComponent + Send + Sync + 'static {
    /// The window was requested to be closed, most likely from the Close Button. Override
    /// this implementation if you want logic in place to prevent a window from closing.
    async fn close_requested(&self) -> KludgineResult<CloseResponse> {
        Ok(CloseResponse::Close)
    }
}

#[derive(Default)]
pub struct WindowBuilder {
    title: Option<String>,
    size: Option<Size>,
    resizable: Option<bool>,
    maximized: Option<bool>,
    visible: Option<bool>,
    transparent: Option<bool>,
    decorations: Option<bool>,
    always_on_top: Option<bool>,
    icon: Option<winit::window::Icon>,
}

impl WindowBuilder {
    pub fn with_title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = Some(resizable);
        self
    }

    pub fn with_maximized(mut self, maximized: bool) -> Self {
        self.maximized = Some(maximized);
        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = Some(transparent);
        self
    }

    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = Some(decorations);
        self
    }

    pub fn with_always_on_top(mut self, always_on_top: bool) -> Self {
        self.always_on_top = Some(always_on_top);
        self
    }

    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl Into<WinitWindowBuilder> for WindowBuilder {
    fn into(self) -> WinitWindowBuilder {
        let mut builder = WinitWindowBuilder::new();
        if let Some(title) = self.title {
            builder = builder.with_title(title);
        }
        if let Some(size) = self.size {
            builder = builder.with_inner_size(size);
        }
        if let Some(resizable) = self.resizable {
            builder = builder.with_resizable(resizable);
        }
        if let Some(maximized) = self.maximized {
            builder = builder.with_maximized(maximized);
        }
        if let Some(visible) = self.visible {
            builder = builder.with_visible(visible);
        }
        if let Some(transparent) = self.transparent {
            builder = builder.with_transparent(transparent);
        }
        if let Some(decorations) = self.decorations {
            builder = builder.with_decorations(decorations);
        }
        if let Some(always_on_top) = self.always_on_top {
            builder = builder.with_always_on_top(always_on_top);
        }

        builder = builder.with_window_icon(self.icon);

        builder
    }
}

#[async_trait]
pub trait OpenableWindow {
    async fn open(window: Self);
}

#[async_trait]
impl<T> OpenableWindow for T
where
    T: Window + WindowCreator<T>,
{
    async fn open(window: Self) {
        Runtime::open_window(Self::get_window_builder(), window).await
    }
}

lazy_static! {
    static ref WINDOW_CHANNELS: KludgineHandle<HashMap<WindowId, Sender<WindowMessage>>> =
        KludgineHandle::new(HashMap::new());
}

lazy_static! {
    static ref WINDOWS: ShardedLock<HashMap<WindowId, RuntimeWindow>> =
        ShardedLock::new(HashMap::new());
}

pub(crate) enum WindowMessage {
    Close,
}

impl WindowMessage {
    pub async fn send_to(self, id: WindowId) -> KludgineResult<()> {
        let sender = {
            let mut channels = WINDOW_CHANNELS.write().await;
            if let Some(sender) = channels.get_mut(&id) {
                sender.clone()
            } else {
                return Err(KludgineError::InternalWindowMessageSendError(
                    "Channel not found for id".to_owned(),
                ));
            }
        };

        sender.send(self).unwrap_or_default();
        Ok(())
    }
}

pub(crate) enum WindowEvent {
    CloseRequested,
    Resize { size: Size, scale_factor: f32 },
    Input(InputEvent),
}

pub(crate) struct RuntimeWindow {
    window: winit::window::Window,
    receiver: Receiver<WindowMessage>,
    event_sender: Sender<WindowEvent>,
    last_known_size: Size,
    last_known_scale_factor: f32,
    keep_running: Arc<AtomicCell<bool>>,
}

impl RuntimeWindow {
    pub(crate) fn open<T>(window_receiver: Receiver<WinitWindow>, app_window: T)
    where
        T: Window + Sized + 'static,
    {
        let window = window_receiver
            .recv()
            .expect("Error receiving winit::window");
        let window_id = window.id();
        let renderer = Renderer::new(&window).expect("Error creating renderer for window");

        let (message_sender, message_receiver) = unbounded();
        let (event_sender, event_receiver) = unbounded();

        let keep_running = Arc::new(AtomicCell::new(true));
        let mut frame_synchronizer = FrameRenderer::run(
            renderer,
            keep_running.clone(),
            window.inner_size().width,
            window.inner_size().height,
        );
        Runtime::spawn(async move {
            frame_synchronizer.relinquish(Frame::default()).await;
            Self::window_main(window_id, frame_synchronizer, event_receiver, app_window).await
        });

        {
            let mut channels = block_on(WINDOW_CHANNELS.write());
            channels.insert(window_id, message_sender);
        }

        let size = window.inner_size();
        let size = Size::new(size.width as f32, size.height as f32);
        let mut runtime_window = Self {
            receiver: message_receiver,
            last_known_size: size,
            keep_running,
            event_sender,
            last_known_scale_factor: window.scale_factor() as f32,
            window,
        };
        runtime_window.notify_size_changed();

        let mut windows = WINDOWS.write().unwrap();
        windows.insert(window_id, runtime_window);
    }

    async fn request_window_close<T>(id: WindowId, ui: &UserInterface<T>) -> KludgineResult<bool>
    where
        T: Window,
    {
        let root_node = global_arena().get(ui.root).await.unwrap();
        let component = root_node.component.read().await;
        let window = component.as_any().downcast_ref::<NodeData<T>>().unwrap();
        if let CloseResponse::Close = window.close_requested().await? {
            WindowMessage::Close.send_to(id).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn window_loop<T>(
        id: WindowId,
        mut frame_synchronizer: FrameSynchronizer,
        event_receiver: Receiver<WindowEvent>,
        window: T,
    ) -> KludgineResult<()>
    where
        T: Window,
    {
        let mut scene = Scene::default();
        let mut ui = UserInterface::new(window, SceneTarget::Scene(scene.clone())).await?;
        #[cfg(feature = "bundled-fonts-enabled")]
        scene.register_bundled_fonts().await;
        let mut interval = tokio::time::interval(Duration::from_nanos(FRAME_DURATION));
        loop {
            while let Some(event) = match event_receiver.try_recv() {
                Ok(event) => Some(event),
                Err(err) => match err {
                    TryRecvError::Empty => None,
                    TryRecvError::Disconnected => return Ok(()),
                },
            } {
                match event {
                    WindowEvent::Resize { size, scale_factor } => {
                        scene.set_internal_size(size).await;
                        scene.set_scale_factor(scale_factor).await;
                    }
                    WindowEvent::CloseRequested => {
                        if Self::request_window_close(id, &ui).await? {
                            return Ok(());
                        }
                    }
                    WindowEvent::Input(input) => {
                        // Notify the window of the raw event, before updaing our internal state
                        ui.process_input(input).await?;

                        if let Event::Keyboard { key, state } = input.event {
                            if let Some(key) = key {
                                let mut scene = scene.data.write().await;
                                match state {
                                    ElementState::Pressed => {
                                        scene.pressed_keys.insert(key);
                                    }
                                    ElementState::Released => {
                                        scene.pressed_keys.remove(&key);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // CHeck for Cmd + W or Alt + f4 to close the window.
            {
                let modifiers = scene.modifiers_pressed().await;
                if modifiers.primary_modifier()
                    && scene.key_pressed(VirtualKeyCode::W).await
                    && Self::request_window_close(id, &ui).await?
                {
                    return Ok(());
                }
            }

            if scene.size().await.area() > 0.0 {
                scene.start_frame().await;
                {
                    let target = SceneTarget::Scene(scene.clone());
                    ui.update(&target).await?;
                    ui.render(&target).await?;
                }
                if let Some(mut frame) = frame_synchronizer.try_take() {
                    frame.update(&scene).await;
                    frame_synchronizer.relinquish(frame).await;
                } else {
                }
            }
            interval.tick().await;
        }
    }

    async fn window_main<T>(
        id: WindowId,
        frame_synchronizer: FrameSynchronizer,
        event_receiver: Receiver<WindowEvent>,
        window: T,
    ) where
        T: Window,
    {
        Self::window_loop(id, frame_synchronizer, event_receiver, window)
            .await
            .expect("Error running window loop.")
    }

    pub(crate) async fn count() -> usize {
        let channels = WINDOW_CHANNELS.read().await;
        channels.len()
    }

    pub(crate) fn process_events(event: &WinitEvent<()>) {
        let mut windows = WINDOWS.write().unwrap();

        if let WinitEvent::WindowEvent { window_id, event } = event {
            // println!("Received event {:?} for window {:?}", event, window_id);
            // println!("Current windows: {:#?}", windows.borrow().keys());
            if let Some(window) = windows.get_mut(&window_id) {
                // println!("Sending event to window {:?}", window.window.id());
                window.process_event(event);
            }
        }

        {
            for window in windows.values_mut() {
                window.receive_messages();
            }
        }

        windows.retain(|_, w| w.keep_running.load());
    }

    pub(crate) fn receive_messages(&mut self) {
        while let Ok(request) = self.receiver.try_recv() {
            match request {
                WindowMessage::Close => {
                    let mut channels = block_on(WINDOW_CHANNELS.write());
                    channels.remove(&self.window.id());
                    self.keep_running.store(false);
                }
            }
        }
    }

    pub(crate) fn process_event(&mut self, event: &WinitWindowEvent) {
        match event {
            WinitWindowEvent::CloseRequested => {
                self.event_sender
                    .send(WindowEvent::CloseRequested)
                    .unwrap_or_default();
            }
            WinitWindowEvent::Resized(size) => {
                self.last_known_size = Size::new(size.width as f32, size.height as f32);
                self.notify_size_changed();
            }
            WinitWindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                self.last_known_scale_factor = *scale_factor as f32;
                self.last_known_size =
                    Size::new(new_inner_size.width as f32, new_inner_size.height as f32);
                self.notify_size_changed();
            }
            WinitWindowEvent::KeyboardInput {
                device_id, input, ..
            } => self
                .event_sender
                .send(WindowEvent::Input(InputEvent {
                    device_id: *device_id,
                    event: Event::Keyboard {
                        key: input.virtual_keycode,
                        state: input.state,
                    },
                }))
                .unwrap_or_default(),
            WinitWindowEvent::MouseInput {
                device_id,
                button,
                state,
                ..
            } => self
                .event_sender
                .send(WindowEvent::Input(InputEvent {
                    device_id: *device_id,
                    event: Event::MouseButton {
                        button: *button,
                        state: *state,
                    },
                }))
                .unwrap_or_default(),
            WinitWindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
                ..
            } => self
                .event_sender
                .send(WindowEvent::Input(InputEvent {
                    device_id: *device_id,
                    event: Event::MouseWheel {
                        delta: *delta,
                        touch_phase: *phase,
                    },
                }))
                .unwrap_or_default(),
            WinitWindowEvent::CursorMoved {
                device_id,
                position,
                ..
            } => self
                .event_sender
                .send(WindowEvent::Input(InputEvent {
                    device_id: *device_id,
                    event: Event::MouseMoved {
                        position: Some(Point::new(
                            position.x as f32 / self.last_known_scale_factor,
                            position.y as f32 / self.last_known_scale_factor,
                        )),
                    },
                }))
                .unwrap_or_default(),
            WinitWindowEvent::CursorLeft { device_id } => self
                .event_sender
                .send(WindowEvent::Input(InputEvent {
                    device_id: *device_id,
                    event: Event::MouseMoved { position: None },
                }))
                .unwrap_or_default(),
            _ => {}
        }
    }

    fn notify_size_changed(&mut self) {
        self.event_sender
            .send(WindowEvent::Resize {
                size: self.last_known_size,
                scale_factor: self.last_known_scale_factor,
            })
            .unwrap_or_default();
    }
}
