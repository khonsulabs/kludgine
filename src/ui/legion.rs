use crate::{
    event::MouseButton,
    math::{Angle, Point, Rect, Scale, Scaled},
    runtime::Runtime,
    scene::SceneTarget,
    shape::Shape,
    sprite::{Sprite, SpriteRotation, SpriteSource},
    ui::{Component, Context, InteractiveComponent, Layout, StyledContext},
    window::EventStatus,
    KludgineResult, RequiresInitialization,
};
use async_channel::Sender;
use async_trait::async_trait;
use crossbeam::atomic::AtomicCell;
use legion::{systems::CommandBuffer, Entity};
use sorted_vec::SortedVec;
use std::{
    cmp::Ordering,
    collections::HashSet,
    fmt::Debug,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub enum UIEvent<Unit, Command> {
    // TODO we don't have a mouse move event structure, just hover() and unhover()
    // MouseMove {
    //     location: Point<f32, Unit>,
    // }
    MouseDown {
        location: Point<f32, Unit>,
        button: MouseButton,
    },
    MouseDrag {
        location: Option<Point<f32, Unit>>,
        button: MouseButton,
    },
    MouseUp {
        location: Option<Point<f32, Unit>>,
        button: MouseButton,
    },
    Command(Command),
}

/// The Canvas component interacts with a Legion world through the
/// `render_drawable` and `render` systems. Schedule the render system
/// after the `render_drawable` system.
#[derive(Debug)]
pub struct Canvas<Unit, Command> {
    systems_handle: RequiresInitialization<SystemsHandle<Unit, Command>>,
    last_camera: CameraState<Unit>,
    current_frame: SortedVec<RenderedDrawable<Unit>>,
}

impl<Unit, Command> Default for Canvas<Unit, Command> {
    fn default() -> Self {
        Self {
            systems_handle: Default::default(),
            last_camera: Default::default(),
            current_frame: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct CanvasFrame<Unit> {
    drawables: Arc<Mutex<SortedVec<RenderedDrawable<Unit>>>>,
}

impl<Unit> Default for CanvasFrame<Unit> {
    fn default() -> Self {
        Self {
            drawables: Arc::new(Mutex::new(SortedVec::new())),
        }
    }
}

impl<Unit> CanvasFrame<Unit> {
    fn insert(&self, drawable: RenderedDrawable<Unit>) {
        let mut drawables = self.drawables.lock().unwrap();
        let drawable = drawable.with_render_id(drawables.len());
        drawables.insert(drawable);
    }
}

#[async_trait]
impl<Unit, Command> Component for Canvas<Unit, Command>
where
    Unit: Clone + Send + Sync + Debug,
    Command: Send + Sync,
{
    async fn render(&self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        let center = layout.inner_bounds().center();
        for rendered in self.current_frame.iter() {
            match &rendered.drawable {
                Drawable::Sprite(sprite) => {
                    let sprite_size = sprite
                        .location()
                        .await
                        .size()
                        .cast_unit::<Unit>()
                        .cast::<f32>()
                        * rendered.scale;

                    let render_location = rendered.center - sprite_size / 2.;
                    sprite
                        .render_within(
                            context.scene(),
                            Rect::new(center + render_location.to_vector(), sprite_size),
                            rendered
                                .rotation
                                .map(|rotation| SpriteRotation::around(rotation, render_location))
                                .unwrap_or_default(),
                        )
                        .await;
                }
                Drawable::Shape(shape) => {
                    let shape = shape.clone() * rendered.scale;
                    shape
                        .render_at(center + rendered.center.to_vector(), context.scene())
                        .await;
                }
            }
        }

        Ok(())
    }

    async fn mouse_down(
        &mut self,
        context: &mut Context,
        window_position: Point<f32, Scaled>,
        button: MouseButton,
    ) -> KludgineResult<EventStatus> {
        let location = (window_position
            - context.last_layout().await.inner_bounds().size.to_vector() / 2.)
            / self.last_camera.scale
            + self.last_camera.look_at.to_vector();
        let _ = self
            .systems_handle
            .sender
            .send(UIEvent::MouseDown { location, button })
            .await;
        Ok(EventStatus::Processed)
    }

    async fn mouse_up(
        &mut self,
        context: &mut Context,
        window_position: Option<Point<f32, Scaled>>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        let last_bounds = context.last_layout().await.inner_bounds();
        let location = window_position.map(|window_position| {
            (window_position - last_bounds.size.to_vector() / 2.) / self.last_camera.scale
                + self.last_camera.look_at.to_vector()
        });
        let _ = self
            .systems_handle
            .sender
            .send(UIEvent::MouseUp { location, button })
            .await;
        Ok(())
    }

    async fn mouse_drag(
        &mut self,
        context: &mut Context,
        window_position: Option<Point<f32, Scaled>>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        let last_bounds = context.last_layout().await.inner_bounds();
        let location = window_position.map(|window_position| {
            (window_position - last_bounds.size.to_vector() / 2.) / self.last_camera.scale
                + self.last_camera.look_at.to_vector()
        });
        let _ = self
            .systems_handle
            .sender
            .send(UIEvent::MouseDrag { location, button })
            .await;
        Ok(())
    }
}

#[async_trait]
impl<Unit, Command> InteractiveComponent for Canvas<Unit, Command>
where
    Unit: Clone + Send + Sync + Debug + 'static,
    Command: Clone + Debug + Send + Sync + 'static,
{
    type Message = ();
    type Command = CanvasCommand<Unit, Command>;
    type Event = ();

    async fn receive_command(
        &mut self,
        context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        match command {
            CanvasCommand::Render(new_frame, new_camera) => {
                self.current_frame = new_frame;
                self.last_camera = new_camera;

                context.set_needs_redraw().await;
            }
            CanvasCommand::ReceiveSystemsHandle(handle) => {
                self.systems_handle.initialize_with(handle);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum CanvasCommand<Unit, Command> {
    ReceiveSystemsHandle(SystemsHandle<Unit, Command>),
    Render(SortedVec<RenderedDrawable<Unit>>, CameraState<Unit>),
}

#[derive(Clone, Debug)]
pub struct RenderedDrawable<Unit> {
    kind: DrawableKind,
    sorting_id: u64,
    render_id: usize,
    drawable: Drawable<Unit>,
    center: Point<f32, Scaled>,
    rotation: Option<Angle>,
    scale: Scale<f32, Unit, Scaled>,
    z: i32,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone)]
enum DrawableKind {
    Shape,
    Sprite,
}

impl<Unit> RenderedDrawable<Unit> {
    pub fn new(drawable: Drawable<Unit>) -> Self {
        let (kind, sorting_id) = drawable.sorting_keys();
        Self {
            kind,
            sorting_id,
            drawable,
            center: Default::default(),
            rotation: None,
            z: 0,
            scale: Scale::new(1.),
            render_id: 0,
        }
    }

    pub fn with_z(mut self, z: i32) -> Self {
        self.z = z;
        self
    }

    pub fn with_center(mut self, center: Point<f32, Scaled>) -> Self {
        self.center = center;
        self
    }

    pub fn with_rotation(mut self, rotation: Angle) -> Self {
        self.rotation = Some(rotation);
        self
    }

    pub fn with_scale(mut self, scale: Scale<f32, Unit, Scaled>) -> Self {
        self.scale = scale;
        self
    }

    fn with_render_id(mut self, render_id: usize) -> Self {
        self.render_id = render_id;
        self
    }
}

impl<Unit> Ord for RenderedDrawable<Unit> {
    /// This implementation of cmp is for ordering within SortedVec.
    /// The ordering is chosen to optimize for Frame's batching operations
    /// by ensuring if a texture is drawn on the same Z level, it is done using
    /// one batched draw call.
    fn cmp(&self, other: &Self) -> Ordering {
        match self.z.cmp(&other.z) {
            Ordering::Equal => match self.kind.cmp(&other.kind) {
                Ordering::Equal => match self.sorting_id.cmp(&other.sorting_id) {
                    Ordering::Equal => self.render_id.cmp(&other.render_id),
                    not_equal => not_equal,
                },
                not_equal => not_equal,
            },

            not_equal => not_equal,
        }
    }
}

impl<Unit> PartialOrd for RenderedDrawable<Unit> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Unit> PartialEq for RenderedDrawable<Unit> {
    fn eq(&self, other: &Self) -> bool {
        self.z.eq(&other.z)
    }
}

impl<Unit> Eq for RenderedDrawable<Unit> {}

#[derive(Clone, Debug)]
pub enum Drawable<Unit> {
    Sprite(SpriteSource),
    Shape(Shape<Unit>),
}

impl<Unit> Drawable<Unit> {
    fn sorting_keys(&self) -> (DrawableKind, u64) {
        match self {
            Drawable::Shape(_) => {
                // Shapes don't have groupings
                (DrawableKind::Shape, 0u64)
            }
            Drawable::Sprite(sprite) => Runtime::block_on(async {
                (DrawableKind::Sprite, sprite.texture().await.id().await)
            }),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ZIndex(pub i32);

#[derive(Clone, Debug)]
pub struct Scaling<Unit>(pub Scale<f32, Unit, Scaled>);

#[derive(Clone, Debug)]
pub struct BatchRender<Unit>(pub Vec<Drawable<Unit>>);

impl<Unit> Default for Scaling<Unit> {
    fn default() -> Self {
        Self(Scale::new(1.))
    }
}

#[legion::system(for_each)]
#[allow(clippy::too_many_arguments)]
fn render_sprite<Unit: Sized + Send + Sync + 'static>(
    sprite: &Sprite,
    entity: &Entity,
    cmd: &mut CommandBuffer,
    #[resource] elapsed: &Option<Duration>,
) {
    let sprite = Runtime::block_on(sprite.get_frame(*elapsed)).unwrap();
    cmd.add_component(*entity, sprite);
}

#[legion::system(for_each)]
#[allow(clippy::too_many_arguments)]
fn render_sprite_source<Unit: Sized + Send + Sync + 'static>(
    sprite: &SpriteSource,
    location: &Point<f32, Unit>,
    rotation: Option<&Angle>,
    scaling: Option<&Scaling<Unit>>,
    z: Option<&ZIndex>,
    #[resource] frame: &CanvasFrame<Unit>,
    #[resource] camera: &CameraState<Unit>,
) {
    let mut drawable = RenderedDrawable::new(Drawable::Sprite(sprite.clone()))
        .with_center((*location - camera.look_at.to_vector()) * camera.scale)
        .with_z(z.cloned().unwrap_or_default().0);
    if let Some(rotation) = rotation {
        drawable = drawable.with_rotation(*rotation);
    }
    if let Some(scaling) = scaling {
        drawable = drawable.with_scale(Scale::new(scaling.0.get() * camera.scale.get()));
    } else {
        drawable = drawable.with_scale(camera.scale);
    }
    frame.insert(drawable);
}

/// queues all entities that have a Drawable component for rendering
#[legion::system(for_each)]
fn render_shape<Unit: Clone + Sized + Send + Sync + 'static>(
    shape: &Shape<Unit>,
    location: &Point<f32, Unit>,
    rotation: Option<&Angle>,
    scaling: Option<&Scaling<Unit>>,
    z: Option<&ZIndex>,
    #[resource] frame: &CanvasFrame<Unit>,
    #[resource] camera: &CameraState<Unit>,
) {
    let mut drawable = RenderedDrawable::new(Drawable::Shape(shape.clone()))
        .with_center((*location - camera.look_at.to_vector()) * camera.scale)
        .with_z(z.cloned().unwrap_or_default().0);

    if let Some(scaling) = scaling {
        drawable = drawable.with_scale(Scale::new(scaling.0.get() * camera.scale.get()));
    } else {
        drawable = drawable.with_scale(camera.scale);
    }

    if let Some(rotation) = rotation {
        drawable = drawable.with_rotation(*rotation);
    }

    frame.insert(drawable);
}

#[legion::system(for_each)]
fn render_batch<Unit: Clone + Sized + Send + Sync + 'static>(
    batch: &BatchRender<Unit>,
    location: &Point<f32, Unit>,
    rotation: Option<&Angle>,
    scaling: Option<&Scaling<Unit>>,
    z: Option<&ZIndex>,
    #[resource] frame: &mut CanvasFrame<Unit>,
    #[resource] camera: &CameraState<Unit>,
) {
    for drawable in batch.0.iter() {
        match drawable {
            Drawable::Sprite(sprite) => {
                render_sprite_source::<Unit>(sprite, location, rotation, scaling, z, frame, camera)
            }
            Drawable::Shape(shape) => {
                render_shape::<Unit>(shape, location, rotation, scaling, z, frame, camera)
            }
        }
    }
}

/// requests the canvas to redraw with all RenderedDrawables
#[legion::system]
fn render<
    Unit: Clone + Debug + Send + Sync + 'static,
    Command: Clone + Debug + Send + Sync + 'static,
>(
    #[resource] canvas: &crate::ui::Entity<Canvas<Unit, Command>>,
    #[resource] frame: &CanvasFrame<Unit>,
    #[resource] camera: &CameraState<Unit>,
) {
    let _ = Runtime::block_on(async move {
        let new_frame = {
            let mut drawables = frame.drawables.lock().unwrap();
            std::mem::take(&mut *drawables)
        };
        canvas
            .send(CanvasCommand::Render(new_frame, camera.clone()))
            .await
    });
}

#[legion::system(for_each)]
fn focus_camera<Unit: Send + Sync + 'static>(
    location: &Point<f32, Unit>,
    _focus: &CameraFocus, // This focuses the query only on the element we are supposed to focus on
    #[resource] camera: &mut CameraState<Unit>,
) {
    // TODO tween the camera's focus within a camera box.. that can be an optional thing on CameraState?
    camera.look_at = *location;
}

pub trait SystemBuilderExt {
    fn add_kludgine_systems<T: LegionSystemsThread>(&mut self) -> &mut Self;
}

impl SystemBuilderExt for legion::systems::Builder {
    fn add_kludgine_systems<T: LegionSystemsThread>(&mut self) -> &mut Self {
        self.flush()
            .add_system(focus_camera_system::<T::Unit>())
            .add_system(render_sprite_system::<T::Unit>())
            .flush()
            .add_system(render_sprite_source_system::<T::Unit>())
            .add_system(render_shape_system::<T::Unit>())
            .add_system(render_batch_system::<T::Unit>())
            .flush()
            .add_system(render_system::<T::Unit, T::Command>())
    }
}

#[derive(Debug, Clone)]
pub struct CameraState<Unit> {
    pub look_at: Point<f32, Unit>,
    pub scale: Scale<f32, Unit, Scaled>,
}

impl<Unit> Default for CameraState<Unit> {
    fn default() -> Self {
        Self {
            look_at: Default::default(),
            scale: Scale::new(1.),
        }
    }
}

pub struct CameraFocus;

#[derive(Debug, Clone)]
pub struct SystemsHandle<Unit, Command> {
    shutdown: Arc<AtomicCell<bool>>,
    sender: Sender<UIEvent<Unit, Command>>,
}

impl<Unit, Command> SystemsHandle<Unit, Command> {
    pub async fn send(&self, command: Command) -> Result<(), async_channel::SendError<Command>> {
        match self.sender.send(UIEvent::Command(command)).await {
            Ok(_) => Ok(()),
            Err(async_channel::SendError(UIEvent::Command(command))) => {
                Err(async_channel::SendError(command))
            }
            _ => unreachable!(),
        }
    }
}

impl<Unit, Command> Drop for SystemsHandle<Unit, Command> {
    fn drop(&mut self) {
        self.shutdown.store(true);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MousePosition<Unit>(pub Point<f32, Unit>);

pub trait LegionSystemsThread: Sized {
    type Unit: Send + Sync + Clone + Debug + 'static;
    type Command: Clone + Debug + Send + Sync + 'static;

    fn initialize(resources: &mut legion::Resources) -> anyhow::Result<Self>;
    fn command_received(
        &mut self,
        command: Self::Command,
        resources: &mut legion::Resources,
    ) -> anyhow::Result<()>;
    fn tick(&mut self, resources: &mut legion::Resources) -> anyhow::Result<()>;

    fn spawn<F: FnOnce() -> legion::Resources + Send + Sync + 'static>(
        canvas: crate::ui::Entity<Canvas<Self::Unit, Self::Command>>,
        scene: &SceneTarget,
        tick_rate: Duration,
        resource_initializer: F,
    ) -> SystemsHandle<Self::Unit, Self::Command> {
        let (sender, receiver) = async_channel::unbounded();
        let shutdown = Arc::new(AtomicCell::new(false));
        let handle = SystemsHandle {
            shutdown: shutdown.clone(),
            sender,
        };
        let handle_for_canvas = handle.clone();
        let canvas_for_task = canvas.clone();
        Runtime::spawn(async move {
            let _ = canvas_for_task
                .send(CanvasCommand::ReceiveSystemsHandle(handle_for_canvas))
                .await;
        })
        .detach();
        let scene = scene.scene_handle();

        std::thread::spawn(move || {
            let mut last_tick_start = None;
            let mut resources = resource_initializer();
            resources.insert(canvas);
            resources.insert(CanvasFrame::<Self::Unit>::default());
            resources.insert(CameraState::<Self::Unit>::default());
            let mut systems_thread = Self::initialize(&mut resources).unwrap();
            resources.insert(HashSet::<MouseButton>::new());
            resources.insert(Option::<MousePosition<Self::Unit>>::None);
            loop {
                if shutdown.load() {
                    break;
                }

                while let Ok(event) = receiver.try_recv() {
                    match event {
                        UIEvent::Command(command) => systems_thread
                            .command_received(command, &mut resources)
                            .unwrap(),
                        UIEvent::MouseDown { location, button } => {
                            resources
                                .get_mut::<HashSet<MouseButton>>()
                                .unwrap()
                                .insert(button);
                            resources.insert(Some(MousePosition::<Self::Unit>(location)));
                        }
                        UIEvent::MouseDrag { location, .. } => {
                            resources.insert(location.map(MousePosition::<Self::Unit>));
                        }
                        UIEvent::MouseUp { location, button } => {
                            resources
                                .get_mut::<HashSet<MouseButton>>()
                                .unwrap()
                                .remove(&button);
                            resources.insert(location.map(MousePosition::<Self::Unit>));
                        }
                    }
                }

                let tick_start = Instant::now();
                let elapsed = last_tick_start
                    .map(|last_tick_start| tick_start.checked_duration_since(last_tick_start))
                    .flatten();

                resources.insert(elapsed);

                // TODO When we support focus, we will want to not report keys pressed when a control has focus if it's not this canvas
                // Can the canvas have focus?
                let keys_pressed = Runtime::block_on(async { scene.keys_pressed().await });
                resources.insert(keys_pressed);

                systems_thread.tick(&mut resources).unwrap();

                last_tick_start = Some(tick_start);

                let now = Instant::now();
                let elapsed = now.checked_duration_since(tick_start).unwrap_or_default();
                let sleep_duration = tick_rate.checked_sub(elapsed).unwrap_or_default();
                std::thread::sleep(sleep_duration);
            }
        });

        handle
    }
}
