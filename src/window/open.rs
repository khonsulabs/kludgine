use std::time::{Duration, Instant};

use async_channel::Sender;
use async_lock::Mutex;

use crate::{
    scene::{Scene, Target},
    KludgineResult,
};

use super::{
    event::{InputEvent, WindowEvent},
    CloseResponse, Window,
};

pub struct OpenWindow<T: Window> {
    window: Mutex<T>,
    redraw_status: Mutex<RedrawStatus>,
    event_sender: Sender<WindowEvent>,
    scene: Target,
}

struct RedrawStatus {
    next_redraw_target: RedrawTarget,
    needs_render: bool,
}

impl<T: Window> OpenWindow<T> {
    pub(crate) fn new(window: T, event_sender: Sender<WindowEvent>, scene: Scene) -> Self {
        Self {
            window: Mutex::new(window),
            event_sender,
            scene: Target {
                scene,
                clip: None,
                offset: None,
            },
            redraw_status: Mutex::new(RedrawStatus {
                needs_render: true,
                next_redraw_target: RedrawTarget::None,
            }),
        }
    }
    pub async fn set_needs_redraw(&self) {
        let mut redraw_status = self.redraw_status.lock().await;
        if !redraw_status.needs_render {
            redraw_status.needs_render = true;
            let _ = self.event_sender.send(WindowEvent::WakeUp).await;
        }
    }

    pub(crate) async fn clear_redraw_target(&self) {
        let mut redraw_status = self.redraw_status.lock().await;
        redraw_status.needs_render = false;
        redraw_status.next_redraw_target = RedrawTarget::None;
    }

    pub(crate) async fn initialize_redraw_target(&self, target_fps: Option<u16>) {
        let mut redraw_status = self.redraw_status.lock().await;
        if let RedrawTarget::None = redraw_status.next_redraw_target {
            match target_fps {
                Some(fps) => {
                    redraw_status.next_redraw_target = RedrawTarget::Scheduled(
                        Instant::now()
                            .checked_add(Duration::from_secs_f32(1. / fps as f32))
                            .unwrap(),
                    );
                }
                None => {
                    redraw_status.next_redraw_target = RedrawTarget::Never;
                }
            }
        }
    }

    pub async fn estimate_next_frame(&self, duration: Duration) {
        self.estimate_next_frame_instant(Instant::now().checked_add(duration).unwrap())
            .await;
    }

    pub async fn estimate_next_frame_instant(&self, instant: Instant) {
        let mut redraw_status = self.redraw_status.lock().await;
        match redraw_status.next_redraw_target {
            RedrawTarget::Never | RedrawTarget::None => {
                redraw_status.next_redraw_target = RedrawTarget::Scheduled(instant);
            }
            RedrawTarget::Scheduled(existing_instant) => {
                if instant < existing_instant {
                    redraw_status.next_redraw_target = RedrawTarget::Scheduled(instant);
                }
            }
        }
    }

    pub(crate) async fn next_redraw_target(&self) -> RedrawTarget {
        let redraw_status = self.redraw_status.lock().await;
        redraw_status.next_redraw_target
    }

    pub async fn needs_render(&self) -> bool {
        let redraw_status = self.redraw_status.lock().await;
        redraw_status.needs_render
            || match redraw_status.next_redraw_target {
                RedrawTarget::Never => false,
                RedrawTarget::None => false,
                RedrawTarget::Scheduled(scheduled_for) => scheduled_for < Instant::now(),
            }
    }

    pub(crate) async fn request_close(&self) -> KludgineResult<CloseResponse> {
        let mut window = self.window.lock().await;
        window.close_requested().await
    }

    pub(crate) async fn process_input(&self, input: InputEvent) -> KludgineResult<()> {
        let mut window = self.window.lock().await;
        window.process_input(input).await
    }

    pub(crate) async fn receive_character(&self, character: char) -> KludgineResult<()> {
        let mut window = self.window.lock().await;
        window.receive_character(character).await
    }

    pub(crate) async fn render(&self) -> KludgineResult<()> {
        let mut window = self.window.lock().await;
        window.render(&self.scene).await?;

        self.clear_redraw_target().await;

        Ok(())
    }

    pub(crate) async fn update(&self, target_fps: Option<u16>) -> KludgineResult<()> {
        self.initialize_redraw_target(target_fps).await;

        let mut window = self.window.lock().await;
        window.update(&self.scene).await
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RedrawTarget {
    None,
    Never,
    Scheduled(Instant),
}

impl Default for RedrawTarget {
    fn default() -> Self {
        Self::None
    }
}

pub(crate) enum UpdateSchedule {
    Now,
    Scheduled(Instant),
}

impl RedrawTarget {
    pub fn next_update_instant(&self) -> Option<UpdateSchedule> {
        match self {
            RedrawTarget::Never => None,
            Self::None => Some(UpdateSchedule::Now),
            Self::Scheduled(scheduled_for) => Some(UpdateSchedule::Scheduled(*scheduled_for)),
        }
    }
}

impl UpdateSchedule {
    pub fn timeout_target(&self) -> Option<Instant> {
        match self {
            UpdateSchedule::Now => None,
            UpdateSchedule::Scheduled(scheduled_for) => {
                if &Instant::now() > scheduled_for {
                    None
                } else {
                    Some(*scheduled_for)
                }
            }
        }
    }
}
