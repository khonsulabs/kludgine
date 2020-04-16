use super::{
    math::{Point, Size},
    runtime::{Runtime, FRAME_DURATION},
    scene::{FlattenedScene, Scene},
    KludgineError, KludgineResult,
};
use async_trait::async_trait;

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use lazy_static::lazy_static;
use rgx::core::*;

use rgx::kit;
use rgx::kit::shape2d;
use rgx::kit::shape2d::{Fill, Shape};
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use winit::{
    event::{
        DeviceId, ElementState, Event as WinitEvent, MouseButton, MouseScrollDelta, TouchPhase,
        VirtualKeyCode, WindowEvent as WinitWindowEvent,
    },
    event_loop::EventLoopWindowTarget,
    window::{WindowBuilder, WindowId},
};

pub enum CloseResponse {
    RemainOpen,
    Close,
}

#[derive(Clone)]
pub struct InputEvent {
    pub device_id: DeviceId,
    pub event: Event,
}

#[derive(Clone)]
pub enum Event {
    Keyboard {
        key: Option<VirtualKeyCode>,
        state: ElementState,
    },
    MouseButton {
        button: MouseButton,
        state: ElementState,
    },
    MouseMoved {
        position: Option<Point>,
    },
    MouseWheel {
        delta: MouseScrollDelta,
        touch_phase: TouchPhase,
    },
}

#[async_trait]
pub trait Window: Send + Sync + 'static {
    async fn close_requested(&self) -> CloseResponse {
        CloseResponse::Close
    }
    async fn initialize(&mut self) {}
    async fn render_2d(&mut self, _scene: &mut Scene) -> KludgineResult<()> {
        Ok(())
    }

    async fn process_input(&mut self, _event: InputEvent) -> KludgineResult<()> {
        Ok(())
    }
}

lazy_static! {
    static ref WINDOW_CHANNELS: Arc<Mutex<HashMap<WindowId, Sender<WindowMessage>>>> =
        { Arc::new(Mutex::new(HashMap::new())) };
}

thread_local! {
    static WINDOWS: RefCell<HashMap<WindowId, RuntimeWindow>> = RefCell::new(HashMap::new());
}

pub(crate) enum WindowMessage {
    Close,
    // UpdateScene(FlattenedScene),
}

impl WindowMessage {
    pub async fn send_to(self, id: WindowId) -> KludgineResult<()> {
        let sender = {
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
    renderer: Renderer,
    swap_chain: SwapChain,
    pipeline: shape2d::Pipeline,
    receiver: Receiver<WindowMessage>,
    event_sender: Sender<WindowEvent>,
    scene: Option<FlattenedScene>,
    should_close: bool,
    wait_for_scene: bool,
    last_known_size: Option<Size>,
    last_known_scale_factor: Option<f32>,
    //mesh_cache: HashMap<Entity, LoadedMesh>,
}

impl RuntimeWindow {
    pub(crate) fn open<T>(
        wb: WindowBuilder,
        event_loop: &EventLoopWindowTarget<()>,
        app_window: Box<T>,
    ) where
        T: Window + ?Sized,
    {
        let window = wb.build(event_loop).expect("Error building window");
        let renderer = Renderer::new(&window).expect("Error creating renderer for window");
        let swap_chain = renderer.swap_chain(
            window.inner_size().width,
            window.inner_size().height,
            PresentMode::NoVsync,
        );
        let pipeline: shape2d::Pipeline = renderer.pipeline(Blending::default());

        let (message_sender, message_receiver) = unbounded();
        let (event_sender, event_receiver) = unbounded();
        Runtime::spawn(Self::window_main::<T>(
            window.id(),
            event_receiver,
            app_window,
        ));

        {
            let mut channels = WINDOW_CHANNELS
                .lock()
                .expect("Error locking window channels map");
            channels.insert(window.id(), message_sender);
        }

        WINDOWS.with(|windows| {
            windows.borrow_mut().insert(
                window.id(),
                Self {
                    window,
                    renderer,
                    swap_chain,
                    pipeline,
                    receiver: message_receiver,
                    scene: None,
                    should_close: false,
                    wait_for_scene: false,
                    last_known_size: None,
                    event_sender,
                    last_known_scale_factor: None,
                    //mesh_cache: HashMap::new(),
                },
            )
        });
    }

    async fn window_loop<T>(
        id: WindowId,
        event_receiver: Receiver<WindowEvent>,
        mut window: Box<T>,
    ) -> KludgineResult<()>
    where
        T: Window + ?Sized,
    {
        let mut scene2d = Scene::new();
        let mut next_frame_target = Instant::now();
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
                        scene2d.size = size;
                        scene2d.scale_factor = scale_factor;
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
                            Event::Keyboard { key, state } => {
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
                    // let mut flattened_scene = FlattenedScene::default();
                    // flattened_scene.flatten_2d(&scene2d);

                    // WindowMessage::UpdateScene(flattened_scene)
                    //     .send_to(id)
                    //     .await?;
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
            } else {
                let sleep_nanos = next_frame_target
                    .checked_duration_since(frame_start)
                    .unwrap_or_default()
                    .as_nanos();
                async_std::task::sleep(Duration::from_nanos(sleep_nanos as u64)).await;
            }
        }
    }

    async fn window_main<T>(id: WindowId, event_receiver: Receiver<WindowEvent>, window: Box<T>)
    where
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
                windows.borrow_mut().retain(|_, w| !w.should_close);
            }
        })
    }

    pub(crate) fn receive_messages(&mut self) {
        while let Ok(request) = self.receiver.try_recv() {
            match request {
                WindowMessage::Close => {
                    let mut channels = WINDOW_CHANNELS
                        .lock()
                        .expect("Error locking window channels map");
                    channels.remove(&self.window.id());
                    self.should_close = true;
                } // WindowMessage::UpdateScene(scene) => {
                  //     self.scene = Some(scene);
                  //     self.wait_for_scene = false;
                  // }
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
                self.last_known_size = Some(Size::new(size.width as f32, size.height as f32));
                self.notify_size_changed();
            }
            WinitWindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                self.last_known_scale_factor = Some(*scale_factor as f32);
                self.last_known_size = Some(Size::new(
                    new_inner_size.width as f32,
                    new_inner_size.height as f32,
                ));
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
                        position: Some(Point::new(position.x as f32, position.y as f32)),
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
                size: self.last_known_size.unwrap_or_default(),
                scale_factor: self.last_known_scale_factor.unwrap_or(1.0),
            })
            .unwrap_or_default();
    }

    pub(crate) fn render_all() {
        WINDOWS.with(|refcell| {
            for window in refcell.borrow_mut().values_mut() {
                window.render().expect("Error rendering window");
            }
        })
    }

    fn render(&mut self) -> KludgineResult<()> {
        if self.last_known_size.is_none() {
            self.last_known_size = Some(self.size());
            self.last_known_scale_factor = Some(self.scale_factor());
            self.notify_size_changed();
        }
        let size = self.window.inner_size();
        let (w, h) = (size.width as u32, size.height as u32);

        if self.swap_chain.width != w || self.swap_chain.height != h {
            self.swap_chain = self.renderer.swap_chain(w, h, PresentMode::NoVsync);
        }

        let (mx, my) = (self.size().width / 2.0, self.size().height / 2.0);

        let buffer = shape2d::Batch::singleton(
            Shape::circle(Point::new(mx, size.height as f32 - my), 20., 32)
                .fill(Fill::Solid(Rgba::new(1., 0., 0., 1.))),
        )
        .finish(&self.renderer);

        let output = self.swap_chain.next();
        let mut frame = self.renderer.frame();

        self.renderer.update_pipeline(
            &self.pipeline,
            kit::ortho(output.width, output.height, Default::default()),
            &mut frame,
        );

        {
            let mut pass = frame.pass(PassOp::Clear(Rgba::TRANSPARENT), &output);

            pass.set_pipeline(&self.pipeline);
            pass.draw_buffer(&buffer);
        }
        self.renderer.present(frame);

        // if let Some(scene) = &self.scene {
        // for mesh in scene.meshes.iter() {
        //     self.mesh_cache
        //         .entry(mesh.original.id)
        //         .and_modify(|lm| lm.update(mesh))
        //         .or_insert_with(|| LoadedMesh::compile(mesh).unwrap())
        //         .render()?;
        // }
        // }
        Ok(())
    }

    pub fn size(&self) -> Size {
        let inner_size = self.window.inner_size();
        Size::new(inner_size.width as f32, inner_size.height as f32)
    }

    pub fn scale_factor(&self) -> f32 {
        self.window.scale_factor() as f32
    }
}
