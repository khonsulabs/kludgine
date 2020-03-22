use crate::internal_prelude::*;
use crate::{
    runtime::{flattened_scene::FlattenedScene, Runtime},
    scene2d::Scene2d,
};
use glutin::{
    event_loop::EventLoop,
    window::{WindowBuilder, WindowId},
    PossiblyCurrent, WindowedContext,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Once},
};
static LOAD_SUPPORT: Once = Once::new();

lazy_static! {
    static ref WINDOW_CHANNELS: Mutex<HashMap<WindowId, mpsc::UnboundedSender<WindowMessage>>> =
        { Mutex::new(HashMap::new()) };
}

thread_local! {
    static WINDOWS: HashMap<WindowId, RuntimeWindow> = HashMap::new();
}

pub(crate) enum WindowMessage {
    Close,
    UpdateScene(FlattenedScene),
}

pub(crate) enum WindowEvent {
    Resize(Size2d),
}

pub struct RuntimeWindow {
    context: WindowedContext<PossiblyCurrent>,
    receiver: mpsc::UnboundedReceiver<WindowMessage>,
    event_sender: mpsc::UnboundedSender<WindowEvent>,
    scene: Option<FlattenedScene>,
    should_close: bool,
    wait_for_scene: bool,
    last_known_size: Option<Size2d>,
}

pub enum CloseResponse {
    RemainOpen,
    Close,
}

#[async_trait]
pub trait Window: Send + Sync {
    async fn new() -> Self;
    async fn close_requested(&self) -> CloseResponse {
        CloseResponse::Close
    }
    async fn initialize(&mut self);
    async fn render_2d(&mut self, _scene: &mut Scene2d) -> KludgineResult<()> {
        Ok(())
    }
}

impl RuntimeWindow {
    pub(crate) fn open<T>(wb: WindowBuilder, event_loop: &EventLoop<()>)
    where
        T: Window + 'static,
    {
        let windowed_context = glutin::ContextBuilder::new()
            .build_windowed(wb, &event_loop)
            .unwrap();
        let context = unsafe { windowed_context.make_current().unwrap() };
        let (message_sender, message_receiver) = mpsc::unbounded();
        let (event_sender, event_receiver) = mpsc::unbounded();
        Runtime::spawn(Self::window_main::<T>(
            context.window().id(),
            event_receiver,
        ));

        {
            let mut channels = WINDOW_CHANNELS
                .lock()
                .expect("Error locking window channels map");
            channels.insert(context.window().id(), message_sender);
        }

        LOAD_SUPPORT.call_once(|| gl::load_with(|s| context.get_proc_address(s) as *const _));

        WINDOWS.with(|windows| {
            windows.insert(
                context.window().id(),
                Self {
                    context,
                    receiver: message_receiver,
                    scene: None,
                    should_close: false,
                    wait_for_scene: false,
                    last_known_size: None,
                    event_sender,
                },
            )
        });
    }

    async fn window_loop<T>(
        id: WindowId,
        event_receiver: mpsc::UnboundedReceiver<WindowEvent>,
    ) -> KludgineResult<()>
    where
        T: Window + 'static,
    {
        let window = T::new().await;
        let mut scene2d = Scene2d::new();
        loop {
            while let Some(event) = event_receiver.try_next().unwrap_or_default() {
                match event {
                    WindowEvent::Resize(size) => {
                        scene2d.size = size;
                    }
                }
            }
            window.render_2d(&mut scene2d).await?;
            let mut flattened_scene = FlattenedScene::default();
            flattened_scene.flatten_2d(&scene2d);

            WindowMessage::UpdateScene(flattened_scene).send().await?;
        }
    }

    async fn window_main<T>(id: WindowId, event_receiver: mpsc::UnboundedReceiver<WindowEvent>)
    where
        T: Window + 'static,
    {
        Self::window_loop::<T>(id, event_receiver)
            .await
            .expect("Error running window loop.")
    }

    pub(crate) fn handle_event(id: WindowId, event: glutin::event::WindowEvent) {
        WINDOWS.with(|windows| {
            if let Some(window) = windows.get(&id) {
                window.process_event(event);
            }
        })
    }

    pub(crate) fn process_event(&mut self, event: glutin::event::Event<()>) {
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

            let size = self.size();
            let size_changed = match &self.last_known_size {
                Some(last_known_size) => last_known_size != &size,
                None => true,
            };

            if size_changed {
                self.wait_for_scene = true;
                self.last_known_size = Some(size);
                block_on(self.event_sender.send(WindowEvent::Resize(size))).unwrap_or_default();
            }
        }
    }

    pub(crate) fn render_all() {
        WINDOWS.with(|windows| {
            for window in windows.iter() {
                window.render();
            }
        })
    }

    fn render(&mut self) {
        let context = self.context.make_current().expect("Error swapping context");
        unsafe {
            gl::ClearColor(0.2, 0.3, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        if !self.wait_for_scene {}
    }

    pub fn size(&self) -> Size2d {
        let inner_size = self.context.window().inner_size();
        Size2d::new(inner_size.width as f32, inner_size.height as f32)
    }
}
