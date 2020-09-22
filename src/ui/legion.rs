use crate::{
    math::Angle,
    math::Rect,
    math::Scaled,
    runtime::Runtime,
    sprite::{SpriteRotation, SpriteSource},
    ui::{Component, Context, InteractiveComponent, Layout, StyledContext},
    KludgineResult,
};
use async_trait::async_trait;
use crossbeam::atomic::AtomicCell;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// The Canvas component interacts with a Legion world through the
/// `render_drawable` and `render` systems. Schedule the render system
/// after the `render_drawable` system.
#[derive(Default, Debug)]
pub struct Canvas {
    current_frame: Vec<Drawable>,
}

#[derive(Default, Debug)]
pub struct CanvasFrame {
    drawables: Vec<Drawable>,
}

#[async_trait]
impl Component for Canvas {
    async fn render(
        &self,
        context: &mut StyledContext,
        _layout: &Layout, // TODO this should be used to offset the camera's viewport (and eventually clip)
    ) -> KludgineResult<()> {
        for drawable in self.current_frame.iter() {
            match drawable {
                Drawable::Sprite {
                    sprite,
                    destination,
                    rotation,
                    ..
                } => {
                    sprite
                        .render_within(
                            context.scene(),
                            *destination,
                            SpriteRotation::around_center(*rotation),
                        )
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
    Render(Vec<Drawable>),
}

#[derive(Clone, Debug)]
pub enum Drawable {
    Sprite {
        sprite: SpriteSource,
        destination: Rect<f32, Scaled>,
        rotation: Angle,
    },
}

/// queues all entities that have a Drawable component for rendering
#[legion::system(for_each)]
pub fn render_drawable(drawable: &Drawable, #[resource] frame: &mut CanvasFrame) {
    frame.drawables.push(drawable.clone());
}

/// requests the canvas to redraw
#[legion::system]
pub fn render(#[resource] canvas: &crate::ui::Entity<Canvas>, #[resource] frame: &mut CanvasFrame) {
    let _ = Runtime::block_on(async move {
        let new_frame = std::mem::take(&mut frame.drawables);
        canvas.send(CanvasCommand::Render(new_frame)).await
    });
}

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
