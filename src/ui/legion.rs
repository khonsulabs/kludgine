use crate::{
    math::{Angle, Point, Rect, Scaled},
    runtime::Runtime,
    shape::Shape,
    sprite::{SpriteRotation, SpriteSource},
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
        _layout: &Layout, // TODO this should be used to offset the camera's viewport (and eventually clip)
    ) -> KludgineResult<()> {
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
                            Rect::new(rendered.center - source_size / 2., source_size),
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
                    shape.render_at(rendered.center, context.scene()).await;
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

/// queues all entities that have a Drawable component for rendering
#[legion::system(for_each)]
pub fn render_drawable(
    drawable: &RenderedDrawable,
    #[resource] frame: &mut CanvasFrame,
    // #[resource] camera: &CameraState,
) {
    frame.drawables.insert(drawable.clone());
}

/// requests the canvas to redraw
#[legion::system]
pub fn render(#[resource] canvas: &crate::ui::Entity<Canvas>, #[resource] frame: &mut CanvasFrame) {
    let _ = Runtime::block_on(async move {
        let new_frame = std::mem::take(&mut frame.drawables);
        canvas.send(CanvasCommand::Render(new_frame)).await
    });
}

// pub struct CameraTracking(legion::Entity);

// pub struct CameraState {
//     look_at: Point<f32, Scaled>,
// }

// #[legion::system]
// pub fn update_camera(
//     #[resource] tracked_entity: &CameraTracking,
//     #[resource] camera: &mut CameraState,
// ) {
// }

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
    fn initialize(canvas: crate::ui::Entity<Canvas>) -> anyhow::Result<Self>;
    fn tick(&mut self, elapsed: Option<Duration>) -> anyhow::Result<()>;
}

pub fn spawn<T: LegionSystemsThread + 'static>(
    canvas: crate::ui::Entity<Canvas>,
    tick_rate: Duration,
) -> SystemsHandle {
    let shutdown_handle = SystemShutdownHandle {
        shutdown: Arc::new(AtomicCell::new(false)),
    };
    let handle = SystemsHandle {
        shutdown_handle: shutdown_handle.clone(),
    };

    std::thread::spawn(move || {
        let mut last_tick_start = None;
        let mut systems_thread = T::initialize(canvas).unwrap();
        loop {
            if shutdown_handle.shutdown.load() {
                break;
            }

            let tick_start = Instant::now();
            let elapsed = last_tick_start
                .map(|last_tick_start| tick_start.checked_duration_since(last_tick_start))
                .flatten();

            systems_thread.tick(elapsed).unwrap();

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
