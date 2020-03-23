use crate::internal_prelude::*;
use crate::{
    runtime::{flattened_scene::FlattenedScene, request::RuntimeRequest, Runtime},
    scene2d::Scene2d,
};
use glutin::{
    event::{Event, WindowEvent as GlutinWindowEvent},
    event_loop::EventLoopWindowTarget,
    window::{WindowBuilder, WindowId},
    NotCurrent, PossiblyCurrent, WindowedContext,
};
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::HashMap,
    ops::Deref,
    sync::{Arc, Mutex, Once},
};
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
    Resize(Size2d),
}

enum TrackedContext {
    Current(WindowedContext<PossiblyCurrent>),
    NotCurrent(WindowedContext<NotCurrent>),
}

impl Deref for TrackedContext {
    type Target = WindowedContext<PossiblyCurrent>;

    fn deref(&self) -> &Self::Target {
        match self {
            TrackedContext::Current(ctx) => ctx,
            TrackedContext::NotCurrent(ctx) => panic!(),
        }
    }
}
impl TrackedContext {
    pub fn window(&self) -> &glutin::window::Window {
        match self {
            TrackedContext::Current(ctx) => ctx.window(),
            TrackedContext::NotCurrent(ctx) => ctx.window(),
        }
    }

    pub fn make_current(self) -> Self {
        match self {
            TrackedContext::Current(_) => {
                panic!("Attempting to make the current context current again")
            }
            TrackedContext::NotCurrent(ctx) => {
                TrackedContext::Current(unsafe { ctx.make_current() }.unwrap())
            }
        }
    }

    pub fn treat_as_not_current(self) -> Self {
        match self {
            TrackedContext::Current(ctx) => {
                TrackedContext::NotCurrent(unsafe { ctx.treat_as_not_current() })
            }
            TrackedContext::NotCurrent(ctx) => TrackedContext::NotCurrent(ctx),
        }
    }

    pub fn resize(&self, size: glutin::dpi::PhysicalSize<u32>) {
        match self {
            TrackedContext::Current(ctx) => ctx.resize(size),
            TrackedContext::NotCurrent(ctx) => panic!(),
        }
    }
}

pub(crate) struct RuntimeWindow {
    context: TrackedContext,
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
pub trait Window: Send + Sync + 'static {
    async fn close_requested(&self) -> CloseResponse {
        CloseResponse::Close
    }
    async fn initialize(&mut self);
    async fn render_2d(&mut self, _scene: &mut Scene2d) -> KludgineResult<()> {
        Ok(())
    }
}

impl RuntimeWindow {
    pub(crate) fn open<T>(wb: WindowBuilder, event_loop: &EventLoopWindowTarget<()>, window: Box<T>)
    where
        T: Window + ?Sized,
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
            window,
        ));

        {
            let mut channels = WINDOW_CHANNELS
                .lock()
                .expect("Error locking window channels map");
            channels.insert(context.window().id(), message_sender);
        }

        LOAD_SUPPORT.call_once(|| gl::load_with(|s| context.get_proc_address(s) as *const _));

        WINDOWS.with(|mut windows| {
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
        loop {
            while let Some(event) = event_receiver.try_next().unwrap_or_default() {
                match event {
                    WindowEvent::Resize(size) => {
                        scene2d.size = size;
                    }
                    WindowEvent::CloseRequested => {
                        if let CloseResponse::Close = window.close_requested().await {
                            WindowMessage::Close.send_to(id).await?;
                            return Ok(());
                        }
                    }
                }
            }
            window.render_2d(&mut scene2d).await?;
            let mut flattened_scene = FlattenedScene::default();
            flattened_scene.flatten_2d(&scene2d);

            WindowMessage::UpdateScene(flattened_scene)
                .send_to(id)
                .await?;
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
                windows.borrow_mut().retain(|k, w| !w.should_close);
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

    pub(crate) fn process_event(&mut self, event: &glutin::event::WindowEvent) {
        match event {
            GlutinWindowEvent::CloseRequested => {
                block_on(self.event_sender.send(WindowEvent::CloseRequested)).unwrap_or_default();
            }
            GlutinWindowEvent::Resized(size) => {}
            GlutinWindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {}
            _ => {}
        }
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

    pub fn size(&self) -> Size2d {
        let inner_size = self.context.window().inner_size();
        Size2d::new(inner_size.width as f32, inner_size.height as f32)
    }
}
