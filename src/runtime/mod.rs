use crate::application::{Application, CloseResponse};
use crate::internal_prelude::*;
use crate::scene2d::Scene2d;
use crate::window::Window;
use futures::{executor::ThreadPool, future::Future};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub(crate) mod flattened_scene;
mod request;
mod threading;
use flattened_scene::FlattenedScene;
use request::*;
use threading::*;

pub struct ApplicationRuntime<App> {
    app: App,
}

impl<App> ApplicationRuntime<App>
where
    App: Application + 'static,
{
    fn launch(
        self,
    ) -> (
        mpsc::UnboundedReceiver<RuntimeRequest>,
        mpsc::UnboundedSender<RuntimeEvent>,
    ) {
        let (event_sender, event_receiver) = mpsc::unbounded();
        let (request_sender, request_receiver) = mpsc::unbounded();
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
        Runtime::spawn(self.async_main());
        (request_receiver, event_sender)
    }

    async fn async_main(mut self)
    where
        App: Application + 'static,
    {
        self.app.initialize();

        self.run()
            .await
            .expect("Error encountered running application loop");
    }

    async fn run(mut self) -> KludgineResult<()>
    where
        App: Application + 'static,
    {
        let mut scene2d = Scene2d::new();
        loop {
            {
                while let Some(event) = {
                    let mut guard = GLOBAL_RUNTIME_RECEIVER
                        .lock()
                        .expect("Error locking runtime reciver");
                    let event_receiver = guard.as_mut().expect("Receiver was not set");
                    event_receiver.try_next().unwrap_or_default()
                } {
                    match event {
                        RuntimeEvent::CloseRequested => {
                            if let CloseResponse::Close = self.app.close_requested().await {
                                return RuntimeRequest::Quit.send().await;
                            }
                        }
                        RuntimeEvent::UpdateDimensions { size } => {
                            scene2d.size = size;
                        }
                    }
                }
            }
            self.app.render_2d(&mut scene2d).await?;
            let mut flattened_scene = FlattenedScene::default();
            flattened_scene.flatten_2d(&scene2d);
            RuntimeRequest::UpdateScene(flattened_scene).send().await?;
        }
    }
}

impl EventProcessor for Runtime {
    fn process_event(
        &mut self,
        event: glutin::event::Event<()>,
        control_flow: &mut glutin::event_loop::ControlFlow,
        window: &mut Window,
    ) {
        let now = Instant::now();
        let mut render_frame = now
            .checked_duration_since(self.next_frame_target)
            .unwrap_or_default()
            .as_nanos()
            > 16_666_667;
        while let Some(request) = self.request_receiver.try_next().unwrap_or_default() {
            match request {
                RuntimeRequest::Quit => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                    return;
                }
                RuntimeRequest::UpdateScene(scene) => {
                    self.current_scene = Some(scene);
                    self.wait_for_scene = false;
                }
            }
        }
        match event {
            glutin::event::Event::WindowEvent { event, .. } => match event {
                glutin::event::WindowEvent::CloseRequested => {
                    block_on(self.event_sender.send(RuntimeEvent::CloseRequested))
                        .unwrap_or_default();
                }
                _ => {}
            },
            _ => {}
        };

        let dimensions = window.size();
        let dimensions_changed = match &self.current_dimensions {
            Some(current_dimensions) => current_dimensions != &dimensions,
            None => true,
        };

        if dimensions_changed {
            self.wait_for_scene = true;
            self.current_dimensions = Some(dimensions);
            block_on(
                self.event_sender
                    .send(RuntimeEvent::UpdateDimensions { size: dimensions }),
            )
            .unwrap_or_default();
        }

        if render_frame && !self.wait_for_scene {
            self.next_frame_target = now + Duration::from_nanos(16_666_667);
            // let mut frame = display.draw();
            // frame.clear_color(0.0, 0.0, 0.0, 1.0); // TODO allow custom background colors
            //                                        // Loop over the flattened scene
            //                                        // - Compile any shaders for materials that are new
            //                                        // - Render in order
            // if let Some(scene) = self.current_scene {
            //     for mesh in scene.meshes.iter() {}
            // }
            // *control_flow = glutin::event_loop::ControlFlow::WaitUntil(self.next_frame_target);
            //frame.finish().expect("Error swapping buffers");
        }
    }
}

/// Runtime is designed to consume the main thread. For cross-platform compatibility, ensure that you call Runtime::run from thee main thread.
pub struct Runtime {
    request_receiver: mpsc::UnboundedReceiver<RuntimeRequest>,
    event_sender: mpsc::UnboundedSender<RuntimeEvent>,
    current_scene: Option<FlattenedScene>,
    mesh_cache: HashMap<generational_arena::Index, LoadedMesh>,
    current_dimensions: Option<Size2d>,
    wait_for_scene: bool,
    next_frame_target: Instant,
}

struct LoadedMesh {
    pub id: generational_arena::Index,

    pub texture: Option<i32>,
}

impl Runtime {
    pub fn new<App>() -> Self
    where
        App: Application + 'static,
    {
        {
            let mut pool_guard = GLOBAL_THREAD_POOL
                .lock()
                .expect("Error locking global thread pool");
            assert!(pool_guard.is_none());
            *pool_guard = Some(ThreadPool::new().expect("Error creating ThreadPool"));
        }
        let app = App::new();
        let app_runtime = ApplicationRuntime { app };
        let (request_receiver, event_sender) = app_runtime.launch();

        Self {
            request_receiver,
            event_sender,
            current_scene: None,
            current_dimensions: None,
            wait_for_scene: false,
            next_frame_target: Instant::now(),
            mesh_cache: HashMap::new(),
        }
    }

    pub fn run(self) {
        // Install the global event handler, and also ensure we aren't trying to initialize two runtimes
        // This is necessary because EventLoop::run requires the function/closure passed to have a `static
        // lifetime for valid reasons. Every approach at using only local variables I could not solve, so
        // we wrap it in a mutex. This abstraction also wraps it in dynamic dispatch, because we can't have
        // a generic-type static variable.
        {
            let mut event_handler = GLOBAL_EVENT_HANDLER
                .lock()
                .expect("Error locking global event handler");
            assert!(event_handler.is_none());
            *event_handler = Some(Box::new(self));
        }

        let event_loop = glutin::event_loop::EventLoop::new();
        let wb = glutin::window::WindowBuilder::new()
            .with_title("Cosmic Verge") // TODO Remove hardcoded name
            .with_inner_size(glutin::dpi::LogicalSize::new(1920.0, 1080.0))
            .with_resizable(true); // TODO remove hardcoded size
        let mut window = crate::window::Window::new(wb, &event_loop);

        event_loop.run(move |event, _, control_flow| {
            let mut event_handler_guard = GLOBAL_EVENT_HANDLER
                .lock()
                .expect("Error locking main event handler");
            let event_handler = event_handler_guard
                .as_mut()
                .expect("No event handler installed");
            event_handler
                .as_mut()
                .process_event(event, control_flow, &mut window);
        });
    }

    pub fn spawn<Fut: Future<Output = ()> + Send + 'static>(future: Fut) {
        let pool = {
            let guard = GLOBAL_THREAD_POOL
                .lock()
                .expect("Error locking thread pool");
            guard.as_ref().expect("No thread pool created yet").clone()
        };
        pool.spawn_ok(future);
    }
}
