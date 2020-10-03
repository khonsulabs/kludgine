use crate::{
    math::{Angle, Point, Rect, Scale, Scaled},
    runtime::Runtime,
    scene::SceneTarget,
    shape::Shape,
    sprite::{Sprite, SpriteRotation, SpriteSource},
    ui::{Component, Context, InteractiveComponent, Layout, StyledContext},
    KludgineResult,
};
use async_channel::Sender;
use async_trait::async_trait;
use crossbeam::atomic::AtomicCell;
use legion::{systems::CommandBuffer, Entity};
use sorted_vec::SortedVec;
use std::{
    cmp::Ordering,
    fmt::Debug,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// The Canvas component interacts with a Legion world through the
/// `render_drawable` and `render` systems. Schedule the render system
/// after the `render_drawable` system.
#[derive(Default, Debug)]
pub struct Canvas<Unit> {
    current_frame: SortedVec<RenderedDrawable<Unit>>,
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
impl<Unit> Component for Canvas<Unit>
where
    Unit: Clone + Send + Sync,
{
    async fn render(&self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        let center = layout.inner_bounds().center();
        for rendered in self.current_frame.iter() {
            match &rendered.drawable {
                Drawable::Sprite(sprite) => {
                    let source_size = sprite
                        .location()
                        .await
                        .size()
                        .cast_unit::<Unit>()
                        .cast::<f32>()
                        * rendered.scale;

                    sprite
                        .render_within(
                            context.scene(),
                            Rect::new(
                                center + rendered.center.to_vector() - source_size / 2.,
                                source_size,
                            ),
                            rendered
                                .rotation
                                .map(|rotation| SpriteRotation::around(rotation, rendered.center))
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
}

#[async_trait]
impl<Unit> InteractiveComponent for Canvas<Unit>
where
    Unit: Clone + Send + Sync + Debug + 'static,
{
    type Message = ();
    type Command = CanvasCommand<Unit>;
    type Event = ();

    async fn receive_command(
        &mut self,
        context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        match command {
            CanvasCommand::Render(new_frame) => {
                self.current_frame = new_frame;

                context.set_needs_redraw().await;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum CanvasCommand<Unit> {
    Render(SortedVec<RenderedDrawable<Unit>>),
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
fn render<Unit: Clone + Debug + Send + Sync + 'static>(
    #[resource] canvas: &crate::ui::Entity<Canvas<Unit>>,
    #[resource] frame: &CanvasFrame<Unit>,
) {
    let _ = Runtime::block_on(async move {
        let new_frame = {
            let mut drawables = frame.drawables.lock().unwrap();
            std::mem::take(&mut *drawables)
        };
        canvas.send(CanvasCommand::Render(new_frame)).await
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
            .add_system(render_system::<T::Unit>())
    }
}

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

#[derive(Debug)]
pub struct SystemsHandle<Command> {
    shutdown: Arc<AtomicCell<bool>>,
    pub sender: Sender<Command>,
}

impl<Command> Drop for SystemsHandle<Command> {
    fn drop(&mut self) {
        self.shutdown.store(true);
    }
}

pub trait LegionSystemsThread: Sized {
    type Unit: Send + Sync + Clone + Debug + 'static;
    type Command: Send + 'static;

    fn initialize(resources: &mut legion::Resources) -> anyhow::Result<Self>;
    fn command_received(
        &mut self,
        command: Self::Command,
        resources: &mut legion::Resources,
    ) -> anyhow::Result<()>;
    fn tick(&mut self, resources: &mut legion::Resources) -> anyhow::Result<()>;

    fn spawn(
        canvas: crate::ui::Entity<Canvas<Self::Unit>>,
        scene: &SceneTarget,
        tick_rate: Duration,
    ) -> SystemsHandle<Self::Command> {
        let (sender, receiver) = async_channel::unbounded();
        let shutdown = Arc::new(AtomicCell::new(false));
        let handle = SystemsHandle {
            shutdown: shutdown.clone(),
            sender,
        };
        let scene = scene.scene_handle();

        std::thread::spawn(move || {
            let mut last_tick_start = None;
            let mut resources = legion::Resources::default();
            resources.insert(canvas);
            resources.insert(CanvasFrame::<Self::Unit>::default());
            resources.insert(CameraState::<Self::Unit>::default());
            let mut systems_thread = Self::initialize(&mut resources).unwrap();
            loop {
                if shutdown.load() {
                    break;
                }

                while let Ok(command) = receiver.try_recv() {
                    systems_thread
                        .command_received(command, &mut resources)
                        .unwrap();
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
