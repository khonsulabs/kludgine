use crate::internal_prelude::*;
use crate::{
    runtime::{flattened_scene::FlattenedScene, Runtime, FRAME_DURATION},
    scene2d::Scene2d,
    window::{CloseResponse, Event as KludgineInputEvent, InputEvent, Window},
};
use glutin::{
    event::{ElementState, Event, WindowEvent as GlutinWindowEvent},
    event_loop::EventLoopWindowTarget,
    window::{WindowBuilder, WindowId},
    GlRequest,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    ops::Deref,
    sync::{Arc, Mutex, Once},
    time::{Duration, Instant},
};

mod loaded_mesh;
mod tracked_context;
use loaded_mesh::LoadedMesh;
use tracked_context::TrackedContext;
static LOAD_SUPPORT: Once = Once::new();

lazy_static! {
    static ref WINDOW_CHANNELS: Arc<Mutex<HashMap<WindowId, mpsc::UnboundedSender<WindowMessage>>>> =
        { Arc::new(Mutex::new(HashMap::new())) };
}

thread_local! {
    static WINDOWS: RefCell<HashMap<WindowId, RuntimeWindow>> = RefCell::new(HashMap::new());
}

pub(crate) enum WindowMessage {
    Close,
    UpdateScene(FlattenedScene),
}

impl WindowMessage {
    pub async fn send_to(self, id: WindowId) -> KludgineResult<()> {
        let mut sender = {
            let mut channels = WINDOW_CHANNELS
                .lock()
                .expect("Error locking window channels");
            if let Some(sender) = channels.get_mut(&id) {
                sender.clone()
            } else {
                return Err(KludgineError::InternalWindowMessageSendError(
                    "Channel not found for id".to_owned(),
                ));
            }
        };

        sender.send(self).await.unwrap_or_default();
        Ok(())
    }
}

pub(crate) enum WindowEvent {
    CloseRequested,
    Resize { size: Size2d, scale_factor: f32 },
    Input(InputEvent),
}

pub(crate) struct RuntimeWindow {
    context: TrackedContext,
    receiver: mpsc::UnboundedReceiver<WindowMessage>,
    event_sender: mpsc::UnboundedSender<WindowEvent>,
    scene: Option<FlattenedScene>,
    should_close: bool,
    wait_for_scene: bool,
    last_known_size: Option<Size2d>,
    last_known_scale_factor: Option<f32>,
    mesh_cache: HashMap<Entity, LoadedMesh>,
}

impl RuntimeWindow {
    pub(crate) fn open<T>(wb: WindowBuilder, event_loop: &EventLoopWindowTarget<()>, window: Box<T>)
    where
        T: Window + ?Sized,
    {
        let windowed_context = glutin::ContextBuilder::new()
            .with_gl(GlRequest::Latest)
            .build_windowed(wb, &event_loop)
            .unwrap();
        let context = unsafe { windowed_context.make_current().unwrap() };
        let (message_sender, message_receiver) = mpsc::unbounded();
        let (event_sender, event_receiver) = mpsc::unbounded();
        Runtime::spawn(Self::window_main::<T>(
            context.window().id(),
            event_receiver,
            window,
        ));

        {
            let mut channels = WINDOW_CHANNELS
                .lock()
                .expect("Error locking window channels map");
            channels.insert(context.window().id(), message_sender);
        }

        LOAD_SUPPORT.call_once(|| gl::load_with(|s| context.get_proc_address(s) as *const _));

        WINDOWS.with(|windows| {
            windows.borrow_mut().insert(
                context.window().id(),
                Self {
                    context: TrackedContext::NotCurrent(unsafe { context.treat_as_not_current() }),
                    receiver: message_receiver,
                    scene: None,
                    should_close: false,
                    wait_for_scene: false,
                    last_known_size: None,
                    event_sender,
                    last_known_scale_factor: None,
                    mesh_cache: HashMap::new(),
                },
            )
        });
    }

    async fn window_loop<T>(
        id: WindowId,
        mut event_receiver: mpsc::UnboundedReceiver<WindowEvent>,
        mut window: Box<T>,
    ) -> KludgineResult<()>
    where
        T: Window + ?Sized,
    {
        let mut scene2d = Scene2d::new();
        let mut next_frame_target = Instant::now();
        loop {
            while let Some(event) = event_receiver.try_next().unwrap_or_default() {
                match event {
                    WindowEvent::Resize { size, scale_factor } => {
                        scene2d.size = size;
                        scene2d.screen_settings.scale_factor = scale_factor;
                    }
                    WindowEvent::CloseRequested => {
                        if let CloseResponse::Close = window.close_requested().await {
                            WindowMessage::Close.send_to(id).await?;
                            return Ok(());
                        }
                    }
                    WindowEvent::Input(input) => {
                        // Notify the window of the raw event, before updaing our internal state
                        window.process_input(input.clone()).await?;

                        match input.event {
                            KludgineInputEvent::Keyboard { key, state } => {
                                if let Some(key) = key {
                                    match state {
                                        ElementState::Pressed => {
                                            scene2d.pressed_keys.insert(key);
                                        }
                                        ElementState::Released => {
                                            scene2d.pressed_keys.remove(&key);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            let frame_start = Instant::now();
            if frame_start
                .checked_duration_since(next_frame_target)
                .unwrap_or_default()
                .as_nanos()
                > 0
            {
                if scene2d.size.width > 0.0 && scene2d.size.height > 0.0 {
                    window.render_2d(&mut scene2d).await?;
                    let mut flattened_scene = FlattenedScene::default();
                    flattened_scene.flatten_2d(&scene2d);

                    WindowMessage::UpdateScene(flattened_scene)
                        .send_to(id)
                        .await?;
                }
                let now = Instant::now();
                let elapsed_nanos = now
                    .checked_duration_since(next_frame_target)
                    .unwrap_or_default()
                    .as_nanos() as i64;
                if next_frame_target < now {
                    next_frame_target = now;
                }
                next_frame_target = next_frame_target
                    .checked_add(Duration::from_nanos(FRAME_DURATION))
                    .unwrap_or(next_frame_target);
                let sleep_nanos = (FRAME_DURATION as i64 - elapsed_nanos).max(0);
                async_std::task::sleep(Duration::from_nanos(sleep_nanos as u64)).await;
            }
        }
    }

    async fn window_main<T>(
        id: WindowId,
        event_receiver: mpsc::UnboundedReceiver<WindowEvent>,
        window: Box<T>,
    ) where
        T: Window + ?Sized,
    {
        Self::window_loop::<T>(id, event_receiver, window)
            .await
            .expect("Error running window loop.")
    }

    pub(crate) fn count() -> usize {
        let channels = WINDOW_CHANNELS
            .lock()
            .expect("Error locking window channels");
        channels.len()
    }

    pub(crate) fn process_events(event: &Event<()>) {
        WINDOWS.with(|windows| {
            match event {
                Event::WindowEvent { window_id, event } => {
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
                windows.borrow_mut().retain(|_, w| !w.should_close);
            }
        })
    }

    pub(crate) fn receive_messages(&mut self) {
        while let Some(request) = self.receiver.try_next().unwrap_or_default() {
            match request {
                WindowMessage::Close => {
                    let mut channels = WINDOW_CHANNELS
                        .lock()
                        .expect("Error locking window channels map");
                    channels.remove(&self.context.window().id());
                    self.should_close = true;
                }
                WindowMessage::UpdateScene(scene) => {
                    self.scene = Some(scene);
                    self.wait_for_scene = false;
                }
            }
        }
    }

    pub(crate) fn process_event(&mut self, event: &glutin::event::WindowEvent) {
        match event {
            GlutinWindowEvent::CloseRequested => {
                block_on(self.event_sender.send(WindowEvent::CloseRequested)).unwrap_or_default();
            }
            GlutinWindowEvent::Resized(size) => {
                self.last_known_size = Some(Size2d::new(size.width as f32, size.height as f32));
                self.notify_size_changed();
            }
            GlutinWindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                self.last_known_scale_factor = Some(*scale_factor as f32);
                self.last_known_size = Some(Size2d::new(
                    new_inner_size.width as f32,
                    new_inner_size.height as f32,
                ));
                self.notify_size_changed();
            }
            GlutinWindowEvent::KeyboardInput {
                device_id, input, ..
            } => block_on(self.event_sender.send(WindowEvent::Input(InputEvent {
                device_id: *device_id,
                event: KludgineInputEvent::Keyboard {
                    key: input.virtual_keycode,
                    state: input.state,
                },
            })))
            .unwrap_or_default(),
            GlutinWindowEvent::MouseInput {
                device_id,
                button,
                state,
                ..
            } => block_on(self.event_sender.send(WindowEvent::Input(InputEvent {
                device_id: *device_id,
                event: KludgineInputEvent::MouseButton {
                    button: *button,
                    state: *state,
                },
            })))
            .unwrap_or_default(),
            GlutinWindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
                ..
            } => block_on(self.event_sender.send(WindowEvent::Input(InputEvent {
                device_id: *device_id,
                event: KludgineInputEvent::MouseWheel {
                    delta: *delta,
                    touch_phase: *phase,
                },
            })))
            .unwrap_or_default(),
            GlutinWindowEvent::CursorMoved {
                device_id,
                position,
                ..
            } => block_on(self.event_sender.send(WindowEvent::Input(InputEvent {
                device_id: *device_id,
                event: KludgineInputEvent::MouseMoved {
                    position: Some(Point2d::new(position.x as f32, position.y as f32)),
                },
            })))
            .unwrap_or_default(),
            GlutinWindowEvent::CursorLeft { device_id } => {
                block_on(self.event_sender.send(WindowEvent::Input(InputEvent {
                    device_id: *device_id,
                    event: KludgineInputEvent::MouseMoved { position: None },
                })))
                .unwrap_or_default()
            }
            _ => {}
        }
    }

    fn notify_size_changed(&mut self) {
        block_on(self.event_sender.send(WindowEvent::Resize {
            size: self.last_known_size.unwrap_or_default(),
            scale_factor: self.last_known_scale_factor.unwrap_or(1.0),
        }))
        .unwrap_or_default();
    }

    pub(crate) fn render_all() {
        WINDOWS.with(|refcell| {
            let windows = refcell.replace(HashMap::new());
            let windows_to_render = windows.into_iter().map(|(_, w)| w).collect::<Vec<_>>();
            let mut last_window: Option<RuntimeWindow> = None;
            let mut finished_windows = HashMap::new();

            for mut window in windows_to_render.into_iter() {
                window.context = window.context.make_current();
                if let Some(mut lw) = last_window {
                    lw.context = lw.context.treat_as_not_current();
                    finished_windows.insert(lw.context.window().id(), lw);
                }

                window
                    .context
                    .deref()
                    .resize(window.context.deref().window().inner_size());

                unsafe {
                    gl::ClearColor(0.0, 0.0, 0.0, 1.0);
                    gl::Clear(gl::COLOR_BUFFER_BIT);
                }

                window.render().expect("Error rendering window");
                assert_eq!(unsafe { gl::GetError() }, 0);

                window.context.deref().swap_buffers().unwrap();
                last_window = Some(window);
            }
            if let Some(mut lw) = last_window {
                lw.context = lw.context.treat_as_not_current();
                finished_windows.insert(lw.context.window().id(), lw);
            }

            refcell.replace(finished_windows);
        })
    }

    fn render(&mut self) -> KludgineResult<()> {
        if self.last_known_size.is_none() {
            self.last_known_size = Some(self.size());
            self.last_known_scale_factor = Some(self.scale_factor());
            self.notify_size_changed();
        }
        if let Some(scene) = &self.scene {
            for mesh in scene.meshes.iter() {
                self.mesh_cache
                    .entry(mesh.original.id)
                    .and_modify(|lm| lm.update(mesh))
                    .or_insert_with(|| LoadedMesh::compile(mesh).unwrap())
                    .render()?;
            }
        }
        Ok(())
    }

    pub fn size(&self) -> Size2d {
        let inner_size = self.context.window().inner_size();
        Size2d::new(inner_size.width as f32, inner_size.height as f32)
    }

    pub fn scale_factor(&self) -> f32 {
        self.context.window().scale_factor() as f32
    }
}
