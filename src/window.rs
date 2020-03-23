use crate::internal_prelude::*;
use crate::{
    materials::prelude::*,
    runtime::{
        flattened_scene::{FlattenedMesh2d, FlattenedScene},
        Runtime,
    },
    scene2d::Scene2d,
};
use cgmath::{prelude::*, Matrix4, Vector4};
use gl::types::*;
use glutin::{
    event::{Event, WindowEvent as GlutinWindowEvent},
    event_loop::EventLoopWindowTarget,
    window::{WindowBuilder, WindowId},
    GlRequest, NotCurrent, PossiblyCurrent, WindowedContext,
};
use std::ptr;
use std::{
    cell::RefCell,
    collections::HashMap,
    ops::Deref,
    sync::{Arc, Mutex, Once},
    time::Duration,
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
    Resize { size: Size2d, scale_factor: f32 },
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
            TrackedContext::NotCurrent(_) => panic!(),
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
    // TODO re-enable cache, see comment near mesh_cache usage later in the file
    // mesh_cache: HashMap<generational_arena::Index, LoadedMesh>,
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
    async fn initialize(&mut self) {}
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
                }
            }
            window.render_2d(&mut scene2d).await?;
            let mut flattened_scene = FlattenedScene::default();
            flattened_scene.flatten_2d(&scene2d);

            WindowMessage::UpdateScene(flattened_scene)
                .send_to(id)
                .await?;
            async_std::task::sleep(Duration::from_millis(1)).await;
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

                window.render();

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

    fn render(&mut self) {
        if self.last_known_size.is_none() {
            self.last_known_size = Some(self.size());
            self.last_known_scale_factor = Some(self.scale_factor());
            self.notify_size_changed();
        }
        use std::ffi::CString;
        if let Some(scene) = &self.scene {
            for mesh in scene.meshes.iter() {
                // TODO We should be able to cache, but for some reason I can't get this to render without recompiling it each time.
                // Something isn't persisting but looking at examples I'm at a loss as to what.
                // let loaded_mesh = self
                //     .mesh_cache
                //     .entry(mesh.mesh.id)
                //     .or_insert_with(|| LoadedMesh::compile(mesh));
                let loaded_mesh = LoadedMesh::compile(mesh);
                let matrix = loaded_mesh.projection;
                let model = loaded_mesh.model;

                loaded_mesh.material.activate();
                unsafe {
                    gl::BindVertexArray(loaded_mesh.vao);
                    gl::UniformMatrix4fv(
                        gl::GetUniformLocation(
                            loaded_mesh.material.shader_program,
                            CString::new("matrix".as_bytes()).unwrap().as_ptr(),
                        ),
                        1,
                        gl::FALSE,
                        matrix.as_ptr() as *const f32,
                    );
                    gl::UniformMatrix4fv(
                        gl::GetUniformLocation(
                            loaded_mesh.material.shader_program,
                            CString::new("model".as_bytes()).unwrap().as_ptr(),
                        ),
                        1,
                        gl::FALSE,
                        model.as_ptr() as *const f32,
                    );
                    gl::Uniform3f(
                        gl::GetUniformLocation(
                            loaded_mesh.material.shader_program,
                            CString::new("offset".as_bytes()).unwrap().as_ptr(),
                        ),
                        loaded_mesh.position.x,
                        loaded_mesh.position.y,
                        loaded_mesh.position.z,
                    );
                    gl::DrawElements(
                        gl::TRIANGLES,
                        loaded_mesh.count,
                        gl::UNSIGNED_INT,
                        ptr::null(),
                    );
                    gl::BindVertexArray(0);
                }
            }
        }
    }

    pub fn size(&self) -> Size2d {
        let inner_size = self.context.window().inner_size();
        Size2d::new(inner_size.width as f32, inner_size.height as f32)
    }

    pub fn scale_factor(&self) -> f32 {
        self.context.window().scale_factor() as f32
    }
}

struct LoadedMesh {
    pub material: CompiledMaterial,
    pub position: Vector4<f32>,
    pub vao: u32,
    pub ebo: u32,
    pub vbo: u32,
    pub count: i32,
    pub projection: Matrix4<f32>,
    pub model: Matrix4<f32>,
}
impl LoadedMesh {
    fn compile(mesh: &FlattenedMesh2d) -> LoadedMesh {
        use std::mem;
        use std::os::raw::c_void;
        let (vao, ebo, vbo, material, count) = {
            let storage = mesh.mesh.storage.lock().expect("Error locking mesh");
            let shape = storage.shape.storage.lock().expect("Error locking shape");
            let vertices: &[Point2d] = &shape.vertices;
            let faces = shape
                .triangles
                .iter()
                .map(|(a, b, c)| (a.0, b.0, c.0))
                .collect::<Vec<(u32, u32, u32)>>();
            let (vao, ebo, vbo) = unsafe {
                let (mut vbo, mut vao, mut ebo) = (0, 0, 0);
                gl::GenVertexArrays(1, &mut vao);
                gl::GenBuffers(1, &mut vbo);
                gl::GenBuffers(1, &mut ebo);
                // bind the Vertex Array Object first, then bind and set vertex buffer(s), and then configure vertex attributes(s).
                gl::BindVertexArray(vao);

                gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (vertices.len() * mem::size_of::<f32>() * 2) as GLsizeiptr,
                    vertices.as_ptr() as *const c_void,
                    gl::STATIC_DRAW,
                );

                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
                gl::BufferData(
                    gl::ELEMENT_ARRAY_BUFFER,
                    (faces.len() * mem::size_of::<u32>() * 3) as GLsizeiptr,
                    faces.as_ptr() as *const c_void,
                    gl::STATIC_DRAW,
                );

                gl::VertexAttribPointer(
                    0,
                    2,
                    gl::FLOAT,
                    gl::FALSE,
                    2 * mem::size_of::<f32>() as GLsizei,
                    ptr::null(),
                );
                gl::EnableVertexAttribArray(0);

                // note that this is allowed, the call to gl::VertexAttribPointer registered VBO as the vertex attribute's bound vertex buffer object so afterwards we can safely unbind
                //gl::BindBuffer(gl::ARRAY_BUFFER, 0);

                // You can unbind the VAO afterwards so other VAO calls won't accidentally modify this VAO, but this rarely happens. Modifying other
                // VAOs requires a call to glBindVertexArray anyways so we generally don't unbind VAOs (nor VBOs) when it's not directly necessary.
                //gl::BindVertexArray(0);

                // uncomment this call to draw in wireframe polygons.
                // gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
                (vao, ebo, vbo)
            };

            let material = storage.material.compile();

            (vao, ebo, vbo, material, faces.len() as i32 * 3)
        };

        LoadedMesh {
            vao,
            ebo,
            vbo,
            count,
            material,
            position: mesh.offset,
            projection: mesh.projection,
            model: mesh.model,
        }
    }
}

impl Drop for LoadedMesh {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vbo);
            self.vbo = 0;
            gl::DeleteBuffers(1, &self.ebo);
            self.ebo = 0;
            gl::DeleteVertexArrays(1, &self.vao);
            self.vao = 0;
        }
    }
}
