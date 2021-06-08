use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use kludgine_core::{
    flume::Sender,
    scene::{Scene, Target},
};

use super::{
    event::{InputEvent, WindowEvent},
    CloseResponse, Window,
};

pub struct OpenWindow<T: Window> {
    window: T,
    pub(crate) redraw_status: RedrawStatus,
    scene: Arc<Scene>,
}

pub struct RedrawStatus {
    next_redraw_target: RedrawTarget,
    needs_render: bool,
    event_sender: Sender<WindowEvent>,
}
impl RedrawStatus {
    pub fn set_needs_redraw(&mut self) {
        if !self.needs_render {
            self.needs_render = true;
            let _ = self.event_sender.send(WindowEvent::WakeUp);
        }
    }

    pub fn estimate_next_frame(&mut self, duration: Duration) {
        self.estimate_next_frame_instant(Instant::now().checked_add(duration).unwrap());
    }

    pub fn estimate_next_frame_instant(&mut self, instant: Instant) {
        match self.next_redraw_target {
            RedrawTarget::Never | RedrawTarget::None => {
                self.next_redraw_target = RedrawTarget::Scheduled(instant);
            }
            RedrawTarget::Scheduled(existing_instant) =>
                if instant < existing_instant {
                    self.next_redraw_target = RedrawTarget::Scheduled(instant);
                },
        }
    }
}

impl<T: Window> OpenWindow<T> {
    pub(crate) fn new(window: T, event_sender: Sender<WindowEvent>, scene: Scene) -> Self {
        Self {
            window,
            scene: Arc::new(scene),
            redraw_status: RedrawStatus {
                needs_render: true,
                next_redraw_target: RedrawTarget::None,
                event_sender,
            },
        }
    }

    pub(crate) fn clear_redraw_target(&mut self) {
        self.redraw_status.needs_render = false;
        self.redraw_status.next_redraw_target = RedrawTarget::None;
    }

    pub(crate) fn initialize_redraw_target(&mut self, target_fps: Option<u16>) {
        if let RedrawTarget::None = self.redraw_status.next_redraw_target {
            match target_fps {
                Some(fps) => {
                    self.redraw_status.next_redraw_target = RedrawTarget::Scheduled(
                        Instant::now()
                            .checked_add(Duration::from_secs_f32(1. / fps as f32))
                            .unwrap(),
                    );
                }
                None => {
                    self.redraw_status.next_redraw_target = RedrawTarget::Never;
                }
            }
        }
    }

    pub(crate) fn next_redraw_target(&self) -> RedrawTarget {
        self.redraw_status.next_redraw_target
    }

    pub fn needs_render(&self) -> bool {
        self.redraw_status.needs_render
            || match self.redraw_status.next_redraw_target {
                RedrawTarget::Never => false,
                RedrawTarget::None => false,
                RedrawTarget::Scheduled(scheduled_for) => scheduled_for < Instant::now(),
            }
    }

    pub(crate) fn request_close(&mut self) -> crate::Result<CloseResponse> {
        self.window.close_requested()
    }

    pub(crate) fn process_input(&mut self, input: InputEvent) -> crate::Result<()> {
        self.window.process_input(input, &mut self.redraw_status)
    }

    pub(crate) fn receive_character(&mut self, character: char) -> crate::Result<()> {
        self.window
            .receive_character(character, &mut self.redraw_status)
    }

    pub(crate) fn initialize(&mut self) -> crate::Result<()> {
        self.window.initialize(&Target {
            scene: self.scene.clone(),
            clip: None,
            offset: None,
        })?;

        Ok(())
    }

    pub(crate) fn render(&mut self) -> crate::Result<()> {
        self.window.render(&Target {
            scene: self.scene.clone(),
            clip: None,
            offset: None,
        })?;

        self.clear_redraw_target();

        Ok(())
    }

    pub(crate) fn update(&mut self, target_fps: Option<u16>) -> crate::Result<()> {
        self.initialize_redraw_target(target_fps);

        self.window.update(
            &Target {
                scene: self.scene.clone(),
                clip: None,
                offset: None,
            },
            &mut self.redraw_status,
        )
    }

    pub(crate) fn scene(&self) -> Target {
        Target {
            scene: self.scene.clone(),
            clip: None,
            offset: None,
        }
    }

    pub(crate) fn scene_mut(&mut self) -> &'_ mut Scene {
        Arc::get_mut(&mut self.scene)
            .expect("Unable to lock scene. Users should not store any references to `Target`")
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
            UpdateSchedule::Scheduled(scheduled_for) =>
                if &Instant::now() > scheduled_for {
                    None
                } else {
                    Some(*scheduled_for)
                },
        }
    }
}
