use crate::{
    math::{Angle, Point, Rect, Scale, Scaled},
    runtime::Runtime,
    shape::Shape,
    sprite::{Sprite, SpriteRotation, SpriteSource},
    ui::{Component, Context, InteractiveComponent, Layout, StyledContext},
    KludgineResult,
};
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
    async fn render(
        &self,
        context: &mut StyledContext,
        layout: &Layout, // TODO this should be used to offset the camera's viewport (and eventually clip)
    ) -> KludgineResult<()> {
        let center = layout.inner_bounds().center();
        for rendered in self.current_frame.iter() {
            match &rendered.drawable {
                Drawable::Sprite(sprite) => {
                    let source_size = sprite
                        .location()
                        .await
                        .size
                        .cast_unit::<Scaled>()
                        .cast::<f32>();

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
    drawable: Drawable,
    center: Point<f32, Scaled>,
    rotation: Option<Angle>,
    z: i32,
}

impl RenderedDrawable {
    pub fn new(drawable: Drawable) -> Self {
        Self {
            drawable,
            center: Default::default(),
            rotation: None,
            z: 0,
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
}

impl Ord for RenderedDrawable {
    fn cmp(&self, other: &Self) -> Ordering {
        self.z.cmp(&other.z)
    }
}

impl PartialOrd for RenderedDrawable {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.z.partial_cmp(&other.z)
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

#[derive(Clone, Debug, Default)]
pub struct ZIndex(pub i32);

/// queues all entities that have a Drawable component for rendering
#[legion::system(for_each)]
pub fn render_sprite<Unit: Sized + Send + Sync + 'static>(
    sprite: &Sprite,
    elapsed: Option<&Duration>,
    location: &Point<f32, Unit>,
    rotation: Option<&Angle>,
    z: Option<&ZIndex>,
    #[resource] frame: &mut CanvasFrame,
    #[resource] camera: &CameraState<Unit>,
) {
    let elapsed = elapsed.cloned();
    let sprite = Runtime::block_on(sprite.get_frame(elapsed)).unwrap();
    let mut drawable = RenderedDrawable::new(Drawable::Sprite(sprite))
        .with_center((*location - camera.look_at.to_vector()) * camera.scale)
        .with_z(z.cloned().unwrap_or_default().0);
    if let Some(rotation) = rotation {
        drawable = drawable.with_rotation(*rotation);
    }
    frame.drawables.insert(drawable);
}

/// queues all entities that have a Drawable component for rendering
#[legion::system(for_each)]
pub fn render_shape<Unit: Clone + Sized + Send + Sync + 'static>(
    shape: &Shape<Unit>,
    location: &Point<f32, Unit>,
    rotation: Option<&Angle>,
    z: Option<&ZIndex>,
    #[resource] frame: &mut CanvasFrame,
    #[resource] camera: &CameraState<Unit>,
) {
    let mut drawable = RenderedDrawable::new(Drawable::Shape(shape.clone() * camera.scale))
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
pub struct SystemsHandle {
    shutdown_handle: SystemShutdownHandle,
}

#[derive(Clone, Debug)]
struct SystemShutdownHandle {
    shutdown: Arc<AtomicCell<bool>>,
}

impl Drop for SystemShutdownHandle {
    fn drop(&mut self) {
        self.shutdown.store(true);
    }
}

pub trait LegionSystemsThread: Sized {
    type Unit: 'static;

    fn initialize(resources: &mut legion::Resources) -> anyhow::Result<Self>;
    fn tick(&mut self, resources: &mut legion::Resources) -> anyhow::Result<()>;

    fn spawn(canvas: crate::ui::Entity<Canvas>, tick_rate: Duration) -> SystemsHandle {
        let shutdown_handle = SystemShutdownHandle {
            shutdown: Arc::new(AtomicCell::new(false)),
        };
        let handle = SystemsHandle {
            shutdown_handle: shutdown_handle.clone(),
        };

        std::thread::spawn(move || {
            let mut last_tick_start = None;
            let mut resources = legion::Resources::default();
            resources.insert(canvas);
            resources.insert(CanvasFrame::default());
            resources.insert(CameraState::<Self::Unit>::default());
            let mut systems_thread = Self::initialize(&mut resources).unwrap();
            loop {
                if shutdown_handle.shutdown.load() {
                    break;
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
                if let Some(sleep_duration) = tick_rate.checked_sub(elapsed) {
                    std::thread::sleep(sleep_duration);
                }
            }
        });

        handle
    }
}
