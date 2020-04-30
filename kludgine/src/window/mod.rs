use super::{
    frame::Frame,
    math::{Point, Size},
    runtime::{Runtime, FRAME_DURATION},
    scene::{Scene, SceneTarget},
    timing::FrequencyLimiter,
    KludgineError, KludgineHandle, KludgineResult,
};
use async_trait::async_trait;

use crossbeam::{
    atomic::AtomicCell,
    channel::{unbounded, Receiver, Sender, TryRecvError},
};
use lazy_static::lazy_static;
use rgx::core::*;

use async_std::sync::RwLock;
use futures::executor::block_on;
use std::{cell::RefCell, collections::HashMap, sync::Arc, time::Duration};
use winit::{
    event::{
        DeviceId, ElementState, Event as WinitEvent, MouseButton, MouseScrollDelta, TouchPhase,
        VirtualKeyCode, WindowEvent as WinitWindowEvent,
    },
    window::{Window as WinitWindow, WindowBuilder as WinitWindowBuilder, WindowId},
};

mod renderer;
use renderer::FrameRenderer;

/// How to react to a request to close a window
pub enum CloseResponse {
    /// Window should remain open
    RemainOpen,
    /// Window should close
    Close,
}

pub enum EventStatus {
    Ignored,
    Processed,
}

/// An Event from a device
#[derive(Clone)]
pub struct InputEvent {
    /// The device that triggered this event
    pub device_id: DeviceId,
    /// The event that was triggered
    pub event: Event,
}

/// An input Event
#[derive(Clone)]
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
pub trait Window: Send + Sync + 'static {
    /// The window was requested to be closed, most likely from the Close Button. Override
    /// this implementation if you want logic in place to prevent a window from closing.
    async fn close_requested(&self) -> KludgineResult<CloseResponse> {
        Ok(CloseResponse::Close)
    }

    /// Called once the Window is opened
    async fn initialize(&mut self, _scene: &mut Scene) -> KludgineResult<()> {
        Ok(())
    }

    /// Called once for each frame, directly before `render`
    async fn update(&mut self, _scene: &mut Scene) -> KludgineResult<()> {
        Ok(())
    }

    /// Called once for each frame of rendering
    async fn render<'a>(&mut self, _scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        Ok(())
    }

    /// An input event occurred for this window
    async fn process_input(&mut self, _event: InputEvent) -> KludgineResult<()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct WindowBuilder {
    title: Option<String>,
    size: Option<Size>,
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

        builder
    }
}

lazy_static! {
    static ref WINDOW_CHANNELS: Arc<RwLock<HashMap<WindowId, Sender<WindowMessage>>>> =
        { Arc::new(RwLock::new(HashMap::new())) };
}

thread_local! {
    static WINDOWS: RefCell<HashMap<WindowId, RuntimeWindow>> = RefCell::new(HashMap::new());
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
    pub(crate) fn open<T>(window: WinitWindow, app_window: Box<T>)
    where
        T: Window + ?Sized,
    {
        let window_id = window.id();
        let renderer = Renderer::new(&window).expect("Error creating renderer for window");

        let (message_sender, message_receiver) = unbounded();
        let (event_sender, event_receiver) = unbounded();
        let frame = Arc::new(RwLock::new(Frame::default()));
        Runtime::spawn(Self::window_main::<T>(
            window_id,
            frame.clone(),
            event_receiver,
            app_window,
        ));

        let keep_running = Arc::new(AtomicCell::new(true));
        FrameRenderer::run(
            renderer,
            frame,
            keep_running.clone(),
            window.inner_size().width,
            window.inner_size().height,
        );

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

        WINDOWS.with(|windows| windows.borrow_mut().insert(window_id, runtime_window));
    }

    async fn request_window_close<T>(id: WindowId, window: &Box<T>) -> KludgineResult<bool>
    where
        T: Window + ?Sized,
    {
        if let CloseResponse::Close = window.close_requested().await? {
            WindowMessage::Close.send_to(id).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn window_loop<T>(
        id: WindowId,
        frame: KludgineHandle<Frame>,
        event_receiver: Receiver<WindowEvent>,
        mut window: Box<T>,
    ) -> KludgineResult<()>
    where
        T: Window + ?Sized,
    {
        let mut scene = Scene::new();
        #[cfg(feature = "bundled-fonts-enabled")]
        scene.register_bundled_fonts().await;
        window.initialize(&mut scene).await?;
        let mut loop_limiter = FrequencyLimiter::new(Duration::from_nanos(FRAME_DURATION));
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
                        scene.set_internal_size(size);
                        scene.set_scale_factor(scale_factor);
                    }
                    WindowEvent::CloseRequested => {
                        if Self::request_window_close(id, &window).await? {
                            return Ok(());
                        }
                    }
                    WindowEvent::Input(input) => {
                        // Notify the window of the raw event, before updaing our internal state
                        window.process_input(input.clone()).await?;

                        match input.event {
                            Event::Keyboard { key, state } => {
                                if let Some(key) = key {
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
                            _ => {}
                        }
                    }
                }
            }
            if let Some(wait_for) = loop_limiter.remaining() {
                async_std::task::sleep(wait_for).await;
            } else {
                // CHeck for Cmd + W or Alt + f4 to close the window.
                let modifiers = scene.modifiers_pressed();
                if modifiers.primary_modifier() && scene.key_pressed(VirtualKeyCode::W) {
                    if Self::request_window_close(id, &window).await? {
                        return Ok(());
                    }
                }
                if scene.size().width > 0.0 && scene.size().height > 0.0 {
                    loop_limiter.advance_frame();
                    scene.start_frame();
                    window.update(&mut scene).await?;
                    window.render(&mut SceneTarget::Scene(&mut scene)).await?;
                    let mut guard = frame.write().await;
                    guard.update(&scene).await;
                }
            }
        }
    }

    async fn window_main<T>(
        id: WindowId,
        frame: KludgineHandle<Frame>,
        event_receiver: Receiver<WindowEvent>,
        window: Box<T>,
    ) where
        T: Window + ?Sized,
    {
        Self::window_loop::<T>(id, frame, event_receiver, window)
            .await
            .expect("Error running window loop.")
    }

    pub(crate) async fn count() -> usize {
        let channels = WINDOW_CHANNELS.read().await;
        channels.len()
    }

    pub(crate) fn process_events(event: &WinitEvent<()>) {
        WINDOWS.with(|windows| {
            match event {
                WinitEvent::WindowEvent { window_id, event } => {
                    if let Some(window) = windows.borrow_mut().get_mut(&window_id) {
                        window.process_event(event);
                    }
                }
                _ => {}
            }

            {
                for window in windows.borrow_mut().values_mut() {
                    window.receive_messages();
                }
            }

            {
                windows.borrow_mut().retain(|_, w| w.keep_running.load());
            }
        })
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
                        // TODO: Convert this point to the proper Y coordinate, figure out which math is right
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
