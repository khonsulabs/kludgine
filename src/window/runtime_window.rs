use crate::{
    event::{ElementState, VirtualKeyCode},
    math::{Pixels, Point, ScreenScale, Size},
    runtime::Runtime,
    scene::Scene,
    ui::{global_arena, NodeData, NodeDataWindowExt, UserInterface},
    window::{
        frame::Frame,
        renderer::{FrameRenderer, FrameSynchronizer},
        CloseResponse, Event, InputEvent, Renderer, Window, WindowEvent, WindowMessage, WINDOWS,
        WINDOW_CHANNELS,
    },
    KludgineError, KludgineResult,
};
use crossbeam::atomic::AtomicCell;
use std::{sync::Arc, time::Instant};

use futures::executor::block_on;
use smol_timeout::TimeoutExt;
use winit::{
    event::{Event as WinitEvent, WindowEvent as WinitWindowEvent},
    window::{Window as WinitWindow, WindowId},
};

pub(crate) struct RuntimeWindow {
    window: winit::window::Window,
    receiver: async_channel::Receiver<WindowMessage>,
    event_sender: async_channel::Sender<WindowEvent>,
    last_known_size: Size,
    last_known_scale_factor: ScreenScale,
    keep_running: Arc<AtomicCell<bool>>,
}

impl RuntimeWindow {
    pub(crate) async fn open<T>(
        window_receiver: async_channel::Receiver<WinitWindow>,
        app_window: T,
    ) where
        T: Window + Sized + 'static,
    {
        let window = window_receiver
            .recv()
            .await
            .expect("Error receiving winit::window");
        let window_id = window.id();
        let instance = easygpu::wgpu::Instance::new(easygpu::wgpu::BackendBit::PRIMARY);
        let renderer = Renderer::new(&instance, &window)
            .await
            .expect("Error creating renderer for window");

        let (message_sender, message_receiver) = async_channel::unbounded();
        let (event_sender, event_receiver) = async_channel::unbounded();

        let keep_running = Arc::new(AtomicCell::new(true));
        let mut frame_synchronizer = FrameRenderer::run(
            renderer,
            keep_running.clone(),
            Size::new(window.inner_size().width, window.inner_size().height),
        );
        let window_event_sender = event_sender.clone();
        Runtime::spawn(async move {
            frame_synchronizer.relinquish(Frame::default()).await;
            Self::window_main(
                window_id,
                frame_synchronizer,
                window_event_sender,
                event_receiver,
                app_window,
            )
            .await
        })
        .detach();

        {
            let mut channels = WINDOW_CHANNELS.write().await;
            channels.insert(window_id, message_sender);
        }

        let size = window.inner_size();
        let size = Size::new(size.width as f32, size.height as f32);
        let mut runtime_window = Self {
            receiver: message_receiver,
            last_known_size: size,
            keep_running,
            event_sender,
            last_known_scale_factor: ScreenScale::new(window.scale_factor() as f32),
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
        let root_node = global_arena().get(&ui.root).await.unwrap();
        let component = root_node.component.read().await;
        let window = component.as_any().downcast_ref::<NodeData<T>>().unwrap();
        if let CloseResponse::Close = window.close_requested().await? {
            WindowMessage::Close.send_to(id).await?;
            return Ok(true);
        }
        Ok(false)
    }

    fn next_window_event_non_blocking(
        event_receiver: &mut async_channel::Receiver<WindowEvent>,
    ) -> KludgineResult<Option<WindowEvent>> {
        match event_receiver.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(async_channel::TryRecvError::Empty) => Ok(None),
            Err(async_channel::TryRecvError::Closed) => Err(
                KludgineError::InternalWindowMessageSendError("Window channel closed".to_owned()),
            ),
        }
    }

    async fn next_window_event_blocking(
        event_receiver: &mut async_channel::Receiver<WindowEvent>,
    ) -> KludgineResult<Option<WindowEvent>> {
        match event_receiver.recv().await {
            Ok(event) => Ok(Some(event)),
            Err(_) => Err(KludgineError::InternalWindowMessageSendError(
                "Window channel closed".to_owned(),
            )),
        }
    }

    async fn next_window_event<T>(
        event_receiver: &mut async_channel::Receiver<WindowEvent>,
        ui: &UserInterface<T>,
    ) -> KludgineResult<Option<WindowEvent>>
    where
        T: Window,
    {
        let needs_render = ui.needs_render().await;
        let next_redraw_target = ui.next_redraw_target().await;
        if needs_render {
            Self::next_window_event_non_blocking(event_receiver)
        } else if let Some(redraw_at) = next_redraw_target.next_update_instant() {
            let timeout_target = redraw_at.timeout_target();
            let remaining_time = timeout_target
                .map(|t| t.checked_duration_since(Instant::now()))
                .flatten();
            if let Some(remaining_time) = remaining_time {
                match Self::next_window_event_blocking(event_receiver)
                    .timeout(remaining_time)
                    .await
                {
                    Some(event) => event,
                    None => Ok(None),
                }
            } else {
                Self::next_window_event_non_blocking(event_receiver)
            }
        } else {
            Self::next_window_event_blocking(event_receiver).await
        }
    }

    async fn window_loop<T>(
        id: WindowId,
        mut frame_synchronizer: FrameSynchronizer,
        event_sender: async_channel::Sender<WindowEvent>,
        mut event_receiver: async_channel::Receiver<WindowEvent>,
        window: T,
    ) -> KludgineResult<()>
    where
        T: Window,
    {
        let mut scene = Scene::new(window.theme());
        let target_fps = window.target_fps();
        let mut ui =
            UserInterface::new(window, scene.clone(), global_arena().clone(), event_sender).await?;
        #[cfg(feature = "bundled-fonts-enabled")]
        scene.register_bundled_fonts().await;
        loop {
            while let Some(event) = match Self::next_window_event(&mut event_receiver, &ui).await {
                Ok(event) => event,
                Err(_) => return Ok(()),
            } {
                match event {
                    WindowEvent::WakeUp => {}
                    WindowEvent::Resize { size, scale_factor } => {
                        scene
                            .set_internal_size(Size::new(size.width as f32, size.height as f32))
                            .await;
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
                    WindowEvent::RedrawRequested => {
                        ui.request_redraw().await;
                    }
                }
            }

            // Check for Cmd + W or Alt + f4 to close the window.
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

                ui.update(&scene, target_fps).await?;

                if ui.needs_render().await {
                    ui.render(&scene).await?;

                    let mut frame = frame_synchronizer.take().await;
                    frame.update(&scene).await;
                    frame_synchronizer.relinquish(frame).await;
                }
            }
        }
    }

    async fn window_main<T>(
        id: WindowId,
        frame_synchronizer: FrameSynchronizer,
        event_sender: async_channel::Sender<WindowEvent>,
        event_receiver: async_channel::Receiver<WindowEvent>,
        window: T,
    ) where
        T: Window,
    {
        Self::window_loop(id, frame_synchronizer, event_sender, event_receiver, window)
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
            if let Some(window) = windows.get_mut(&window_id) {
                window.process_event(event);
            }
        } else if let WinitEvent::RedrawRequested(window_id) = event {
            if let Some(window) = windows.get_mut(&window_id) {
                window.request_redraw();
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

    pub(crate) fn request_redraw(&self) {
        self.event_sender
            .try_send(WindowEvent::RedrawRequested)
            .unwrap_or_default();
    }

    pub(crate) fn process_event(&mut self, event: &WinitWindowEvent) {
        match event {
            WinitWindowEvent::CloseRequested => {
                self.event_sender
                    .try_send(WindowEvent::CloseRequested)
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
                self.last_known_scale_factor = ScreenScale::new(*scale_factor as f32);
                self.last_known_size =
                    Size::new(new_inner_size.width as f32, new_inner_size.height as f32);
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
                            Point::from_lengths(
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
