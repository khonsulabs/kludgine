use crate::{
    math::{Angle, Point, Rect, Scale, Scaled},
    runtime::Runtime,
    shape::Shape,
    sprite::{Sprite, SpriteRotation, SpriteSource},
    ui::{Component, Context, InteractiveComponent, Layout, StyledContext},
    KludgineResult,
};
use async_channel::Sender;
use async_trait::async_trait;
use crossbeam::atomic::AtomicCell;
use sorted_vec::SortedVec;
use std::{
    cmp::Ordering,
    sync::Arc,
    time::{Duration, Instant},
};

/// The Canvas component interacts with a Legion world through the
/// `render_drawable` and `render` systems. Schedule the render system
/// after the `render_drawable` system.
#[derive(Default, Debug)]
pub struct Canvas {
    current_frame: SortedVec<RenderedDrawable>,
}

#[derive(Default, Debug)]
pub struct CanvasFrame {
    drawables: SortedVec<RenderedDrawable>,
}

#[async_trait]
impl Component for Canvas {
    async fn render(&self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        let center = layout.inner_bounds().center();
        for rendered in self.current_frame.iter() {
            match &rendered.drawable {
                Drawable::Sprite(sprite) => {
                    let source_size = sprite
                        .location()
                        .await
                        .size
                        .cast_unit::<Scaled>()
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
                    assert!(
                        rendered.rotation.is_none(),
                        "TODO Need to implement rotated shapes"
                    );
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
impl InteractiveComponent for Canvas {
    type Message = ();
    type Command = CanvasCommand;
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
pub enum CanvasCommand {
    Render(SortedVec<RenderedDrawable>),
}

#[derive(Clone, Debug)]
pub struct RenderedDrawable {
    kind: DrawableKind,
    sorting_id: u64,
    render_id: usize,
    drawable: Drawable,
    center: Point<f32, Scaled>,
    rotation: Option<Angle>,
    scale: f32,
    z: i32,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone)]
enum DrawableKind {
    Shape,
    Sprite,
}

impl RenderedDrawable {
    pub fn new(drawable: Drawable, render_id: usize) -> Self {
        let (kind, sorting_id) = drawable.sorting_keys();
        Self {
            kind,
            sorting_id,
            render_id,
            drawable,
            center: Default::default(),
            rotation: None,
            z: 0,
            scale: 1.,
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

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }
}

impl Ord for RenderedDrawable {
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

impl PartialOrd for RenderedDrawable {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for RenderedDrawable {
    fn eq(&self, other: &Self) -> bool {
        self.z.eq(&other.z)
    }
}

impl Eq for RenderedDrawable {}

#[derive(Clone, Debug)]
pub enum Drawable {
    Sprite(SpriteSource),
    Shape(Shape<Scaled>),
}

impl Drawable {
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
pub struct Scaling(pub f32);

impl Default for Scaling {
    fn default() -> Self {
        Self(1.)
    }
}

/// queues all entities that have a Drawable component for rendering
// TODO: Split this system into two systems, first does the sprite update, and a second system renders a SpriteSource component with the rest of the values
#[legion::system(for_each)]
#[allow(clippy::too_many_arguments)]
pub fn render_sprite<Unit: Sized + Send + Sync + 'static>(
    sprite: &Sprite,
    elapsed: Option<&Duration>,
    location: &Point<f32, Unit>,
    rotation: Option<&Angle>,
    scaling: Option<&Scaling>,
    z: Option<&ZIndex>,
    #[resource] frame: &mut CanvasFrame,
    #[resource] camera: &CameraState<Unit>,
) {
    let elapsed = elapsed.cloned();
    let sprite = Runtime::block_on(sprite.get_frame(elapsed)).unwrap();
    let mut drawable = RenderedDrawable::new(Drawable::Sprite(sprite), frame.drawables.len())
        .with_center((*location - camera.look_at.to_vector()) * camera.scale)
        .with_z(z.cloned().unwrap_or_default().0);
    if let Some(rotation) = rotation {
        drawable = drawable.with_rotation(*rotation);
    }
    if let Some(scaling) = scaling {
        drawable = drawable.with_scale(scaling.0);
    }
    frame.drawables.insert(drawable);
}

/// queues all entities that have a Drawable component for rendering
// TODO: Investigate change tracking #[filter(maybe_changed::<Position>())]
#[legion::system(for_each)]
pub fn render_shape<Unit: Clone + Sized + Send + Sync + 'static>(
    shape: &Shape<Unit>,
    location: &Point<f32, Unit>,
    rotation: Option<&Angle>,
    z: Option<&ZIndex>,
    #[resource] frame: &mut CanvasFrame,
    #[resource] camera: &CameraState<Unit>,
) {
    let mut drawable = RenderedDrawable::new(
        Drawable::Shape(shape.clone() * camera.scale),
        frame.drawables.len(),
    )
    .with_center((*location - camera.look_at.to_vector()) * camera.scale)
    .with_z(z.cloned().unwrap_or_default().0);
    if let Some(rotation) = rotation {
        drawable = drawable.with_rotation(*rotation);
    }
    frame.drawables.insert(drawable);
}

/// requests the canvas to redraw
#[legion::system]
pub fn render(#[resource] canvas: &crate::ui::Entity<Canvas>, #[resource] frame: &mut CanvasFrame) {
    let _ = Runtime::block_on(async move {
        let new_frame = std::mem::take(&mut frame.drawables);
        canvas.send(CanvasCommand::Render(new_frame)).await
    });
}

#[legion::system(for_each)]
pub fn focus_camera<Unit: Sized + Send + Sync + 'static>(
    location: &Point<f32, Unit>,
    _focus: &CameraFocus, // This focuses the query only on the element we are supposed to focus on
    #[resource] camera: &mut CameraState<Unit>,
) {
    // TODO tween the camera's focus within a camera box.. that can be an optional thing on CameraState?
    camera.look_at = *location;
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
    type Unit: 'static;
    type Command: Send + 'static;

    fn initialize(resources: &mut legion::Resources) -> anyhow::Result<Self>;
    fn command_received(
        &mut self,
        command: Self::Command,
        resources: &mut legion::Resources,
    ) -> anyhow::Result<()>;
    fn tick(&mut self, resources: &mut legion::Resources) -> anyhow::Result<()>;

    fn spawn(
        canvas: crate::ui::Entity<Canvas>,
        tick_rate: Duration,
    ) -> SystemsHandle<Self::Command> {
        let (sender, receiver) = async_channel::unbounded();
        let shutdown = Arc::new(AtomicCell::new(false));
        let handle = SystemsHandle {
            shutdown: shutdown.clone(),
            sender,
        };

        std::thread::spawn(move || {
            let mut last_tick_start = None;
            let mut resources = legion::Resources::default();
            resources.insert(canvas);
            resources.insert(CanvasFrame::default());
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
                if let Some(elapsed) = last_tick_start
                    .map(|last_tick_start| tick_start.checked_duration_since(last_tick_start))
                    .flatten()
                {
                    resources.insert(elapsed);
                } else {
                    resources.remove::<Duration>();
                }

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
