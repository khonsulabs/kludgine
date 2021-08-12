use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use kludgine_core::{
    easygpu::prelude::*,
    flume,
    math::{Pixels, Point, Raw, ScreenScale, Size},
    scene::{Scene, SceneEvent},
    winit::{
        self,
        event::WindowEvent as WinitWindowEvent,
        window::{Theme, WindowId},
    },
    FrameRenderer,
};
use once_cell::sync::OnceCell;
use tracing::instrument;

use super::OpenWindow;
use crate::{
    runtime::{Runtime, RuntimeRequest, WINDOWS},
    window::{
        event::{ElementState, Event, InputEvent, VirtualKeyCode, WindowEvent},
        CloseResponse, Window, WindowMessage, WINDOW_CHANNELS,
    },
    Error,
};

pub struct RuntimeWindow {
    pub window_id: WindowId,
    pub keep_running: Arc<AtomicBool>,
    receiver: flume::Receiver<WindowMessage>,
    event_sender: flume::Sender<WindowEvent>,
    last_known_size: Size<u32, Raw>,
    last_known_scale_factor: ScreenScale,
}

pub struct RuntimeWindowConfig {
    window_id: WindowId,
    instance: wgpu::Instance,
    surface: wgpu::Surface,
    initial_size: Size<u32, Raw>,
    scale_factor: f32,
}

impl RuntimeWindowConfig {
    pub fn new(window: &winit::window::Window) -> Self {
        // TODO in wasm, we need to explicity enable GL, but since wasm isn't possible
        // right now, we're just hardcoding primary
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        Self {
            window_id: window.id(),
            instance,
            surface,
            initial_size: Size::new(window.inner_size().width, window.inner_size().height),
            scale_factor: window.scale_factor() as f32,
        }
    }
}

static OPENED_FIRST_WINDOW: OnceCell<()> = OnceCell::new();

pub fn opened_first_window() -> bool {
    OPENED_FIRST_WINDOW.get().is_some()
}

fn set_opened_first_window() {
    OPENED_FIRST_WINDOW.get_or_init(|| ());
}

#[cfg(not(target_arch = "wasm32"))]
type Format = kludgine_core::sprite::Srgb;
#[cfg(target_arch = "wasm32")]
type Format = kludgine_core::sprite::Normal;

impl RuntimeWindow {
    pub(crate) fn open<T>(
        window_receiver: &flume::Receiver<RuntimeWindowConfig>,
        initial_system_theme: Theme,
        app_window: T,
    ) where
        T: Window + Sized + 'static,
    {
        let RuntimeWindowConfig {
            window_id,
            instance,
            surface,
            initial_size,
            scale_factor,
        } = window_receiver
            .recv()
            .expect("Error receiving winit::window");

        let (message_sender, message_receiver) = flume::unbounded();
        let (event_sender, event_receiver) = flume::unbounded();

        let keep_running = Arc::new(AtomicBool::new(true));
        let task_keep_running = keep_running.clone();
        let window_event_sender = event_sender.clone();
        let (scene_event_sender, scene_event_receiver) = flume::unbounded();

        // Each window has its own thread/task operating its core update/render
        // lifecycle.
        std::thread::Builder::new()
            .name(String::from("kludgine-window-loop"))
            .spawn(move || {
                Self::window_main(
                    window_id,
                    scene_event_sender,
                    window_event_sender,
                    event_receiver,
                    initial_system_theme,
                    app_window,
                );
            })
            .unwrap();

        let renderer = Runtime::block_on(async move {
            Renderer::for_surface(surface, &instance)
                .await
                .expect("Error creating renderer for window")
        });

        FrameRenderer::<Format>::run(renderer, task_keep_running, scene_event_receiver, || {
            RuntimeRequest::WindowClosed.send();
        });

        {
            let mut channels = WINDOW_CHANNELS.lock().unwrap();
            channels.insert(window_id, message_sender);
        }

        let mut runtime_window = Self {
            receiver: message_receiver,
            last_known_size: initial_size,
            keep_running,
            event_sender,
            last_known_scale_factor: ScreenScale::new(scale_factor),
            window_id,
        };
        runtime_window.notify_size_changed();

        let mut windows = WINDOWS.write().unwrap();
        windows.insert(window_id, runtime_window);

        set_opened_first_window();
    }

    fn request_window_close<T>(id: WindowId, window: &mut OpenWindow<T>) -> crate::Result<bool>
    where
        T: Window,
    {
        if let CloseResponse::RemainOpen = window.request_close()? {
            return Ok(false);
        }

        WindowMessage::Close.send_to(id)?;
        Ok(true)
    }

    fn next_window_event_non_blocking(
        event_receiver: &mut flume::Receiver<WindowEvent>,
    ) -> crate::Result<Option<WindowEvent>> {
        match event_receiver.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(flume::TryRecvError::Empty) => Ok(None),
            Err(flume::TryRecvError::Disconnected) => Err(Error::InternalWindowMessageSend(
                "Window channel closed".to_owned(),
            )),
        }
    }

    fn next_window_event_blocking(
        event_receiver: &mut flume::Receiver<WindowEvent>,
    ) -> crate::Result<Option<WindowEvent>> {
        match event_receiver.recv() {
            Ok(event) => Ok(Some(event)),
            Err(_) => Err(Error::InternalWindowMessageSend(
                "Window channel closed".to_owned(),
            )),
        }
    }

    fn next_window_event_timeout(
        event_receiver: &mut flume::Receiver<WindowEvent>,
        wait_for: Duration,
    ) -> crate::Result<Option<WindowEvent>> {
        match event_receiver.recv_timeout(wait_for) {
            Ok(event) => Ok(Some(event)),
            Err(flume::RecvTimeoutError::Timeout) => Ok(None),
            Err(_) => Err(Error::InternalWindowMessageSend(
                "Window channel closed".to_owned(),
            )),
        }
    }

    fn next_window_event<T>(
        event_receiver: &mut flume::Receiver<WindowEvent>,
        window: &OpenWindow<T>,
    ) -> crate::Result<Option<WindowEvent>>
    where
        T: Window,
    {
        #[cfg(target_arch = "wasm32")]
        {
            // On wasm, the browser is controlling our async runtime, and we don't have a
            // good way to do a Timeout-style function This shouldn't have any noticable
            // effects, because the browser will throttle frames for us automatically
            // We could refactor to trigger redraws separately from winit, in which case we
            // could properly use the frame update logic
            Self::next_window_event_non_blocking(event_receiver)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let next_redraw_target = window.next_redraw_target();
            if !window.can_wait_for_events() {
                Self::next_window_event_non_blocking(event_receiver)
            } else if let Some(redraw_at) = next_redraw_target.next_update_instant() {
                let timeout_target = redraw_at.timeout_target();
                let remaining_time =
                    timeout_target.and_then(|t| t.checked_duration_since(Instant::now()));
                if let Some(remaining_time) = remaining_time {
                    Self::next_window_event_timeout(event_receiver, remaining_time)
                } else {
                    // No remaining time, only process events that have already arrived.
                    Self::next_window_event_non_blocking(event_receiver)
                }
            } else {
                Self::next_window_event_blocking(event_receiver)
            }
        }
    }

    fn window_loop<T>(
        id: WindowId,
        scene_event_sender: flume::Sender<SceneEvent>,
        event_sender: flume::Sender<WindowEvent>,
        mut event_receiver: flume::Receiver<WindowEvent>,
        initial_system_theme: Theme,
        window: T,
    ) -> crate::Result<()>
    where
        T: Window,
    {
        let scene = Scene::new(scene_event_sender, initial_system_theme);

        let target_fps = window.target_fps();
        let mut window = OpenWindow::new(window, event_sender, scene);

        window.initialize()?;
        loop {
            while let Some(event) = match Self::next_window_event(&mut event_receiver, &window) {
                Ok(event) => event,
                Err(_) => return Ok(()),
            } {
                match event {
                    WindowEvent::WakeUp => {
                        window.redraw_status.set_needs_update();
                    }
                    WindowEvent::SystemThemeChanged(system_theme) => {
                        window.scene_mut().set_system_theme(system_theme);
                        window.redraw_status.set_needs_redraw();
                    }
                    WindowEvent::Resize { size, scale_factor } => {
                        window.scene_mut().set_size(size.cast_unit());
                        window.scene_mut().set_scale_factor(scale_factor);
                    }
                    WindowEvent::CloseRequested =>
                        if Self::request_window_close(id, &mut window)? {
                            return Ok(());
                        },
                    WindowEvent::Input(input) => {
                        if let Event::Keyboard {
                            key: Some(key),
                            state,
                            ..
                        } = input.event
                        {
                            match state {
                                ElementState::Pressed => {
                                    window.scene_mut().keys_pressed.insert(key);
                                }
                                ElementState::Released => {
                                    window.scene_mut().keys_pressed.remove(&key);
                                }
                            }
                        }

                        window.process_input(input)?;
                    }
                    WindowEvent::ReceiveCharacter(character) => {
                        window.receive_character(character)?;
                    }
                    WindowEvent::RedrawRequested => {
                        window.redraw_status.set_needs_redraw();
                    }
                }
            }

            // Check for Cmd + W or Alt + f4 to close the window.
            {
                let modifiers = window.scene().modifiers_pressed();
                if modifiers.primary_modifier()
                    && window.scene().keys_pressed.contains(&VirtualKeyCode::W)
                    && Self::request_window_close(id, &mut window)?
                {
                    return Ok(());
                }
            }

            if window.scene().size().area().get() > 0.0 {
                window.scene_mut().start_frame();

                window.update(target_fps)?;

                if window.should_redraw_now() {
                    window.render()?;
                    window.scene_mut().end_frame();
                }
            }
        }
    }

    fn window_main<T>(
        id: WindowId,
        scene_event_sender: flume::Sender<SceneEvent>,
        event_sender: flume::Sender<WindowEvent>,
        event_receiver: flume::Receiver<WindowEvent>,
        initial_system_theme: Theme,
        window: T,
    ) where
        T: Window,
    {
        Self::window_loop(
            id,
            scene_event_sender,
            event_sender,
            event_receiver,
            initial_system_theme,
            window,
        )
        .expect("Error running window loop.");
    }

    pub(crate) fn count() -> usize {
        if opened_first_window() {
            let channels = WINDOW_CHANNELS.lock().unwrap();
            channels.len()
        } else {
            // If our first window hasn't opened, return a count of 1. This will happen in a
            // single-threaded environment because RuntimeWindow::open is spawned, not
            // blocked_on.
            1
        }
    }

    pub(crate) fn receive_messages(&mut self) {
        while let Ok(request) = self.receiver.try_recv() {
            match request {
                WindowMessage::Close => {
                    let mut channels = WINDOW_CHANNELS.lock().unwrap();
                    channels.remove(&self.window_id);
                    self.keep_running.store(false, Ordering::SeqCst);
                }
            }
        }
    }

    pub(crate) fn request_redraw(&self) {
        self.event_sender
            .try_send(WindowEvent::RedrawRequested)
            .unwrap_or_default();
    }

    #[instrument(name = "RuntimeWindow::process_event", level = "trace", skip(self))]
    pub(crate) fn process_event(&mut self, event: &WinitWindowEvent<'_>) {
        match event {
            WinitWindowEvent::CloseRequested => {
                self.event_sender
                    .try_send(WindowEvent::CloseRequested)
                    .unwrap_or_default();
            }
            WinitWindowEvent::Resized(size) => {
                self.last_known_size = Size::new(size.width, size.height);
                self.notify_size_changed();
            }
            WinitWindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                self.last_known_scale_factor = ScreenScale::new(*scale_factor as f32);
                self.last_known_size = Size::new(new_inner_size.width, new_inner_size.height);
                self.notify_size_changed();
            }
            WinitWindowEvent::KeyboardInput {
                device_id, input, ..
            } => self
                .event_sender
                .try_send(WindowEvent::Input(InputEvent {
                    device_id: *device_id,
                    event: Event::Keyboard {
                        key: input.virtual_keycode,
                        state: input.state,
                        scancode: input.scancode,
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
                .try_send(WindowEvent::Input(InputEvent {
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
                .try_send(WindowEvent::Input(InputEvent {
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
                .try_send(WindowEvent::Input(InputEvent {
                    device_id: *device_id,
                    event: Event::MouseMoved {
                        position: Some(
                            Point::from_figures(
                                Pixels::new(position.x as f32),
                                Pixels::new(position.y as f32),
                            ) / self.last_known_scale_factor,
                        ),
                    },
                }))
                .unwrap_or_default(),
            WinitWindowEvent::CursorLeft { device_id } => self
                .event_sender
                .try_send(WindowEvent::Input(InputEvent {
                    device_id: *device_id,
                    event: Event::MouseMoved { position: None },
                }))
                .unwrap_or_default(),
            WinitWindowEvent::ReceivedCharacter(character) => self
                .event_sender
                .try_send(WindowEvent::ReceiveCharacter(*character))
                .unwrap_or_default(),
            WinitWindowEvent::ThemeChanged(theme) => self
                .event_sender
                .try_send(WindowEvent::SystemThemeChanged(*theme))
                .unwrap_or_default(),
            _ => {}
        }
    }

    fn notify_size_changed(&mut self) {
        self.event_sender
            .try_send(WindowEvent::Resize {
                size: self.last_known_size,
                scale_factor: self.last_known_scale_factor,
            })
            .unwrap_or_default();
    }
}
