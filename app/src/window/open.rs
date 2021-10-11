use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use kludgine_core::{
    figures::{Points, Scaled},
    flume::Sender,
    math::Scale,
    scene::{Scene, Target},
    winit::window::WindowId,
};

use super::{
    event::{InputEvent, WindowEvent},
    CloseResponse, Window,
};
use crate::WindowHandle;

pub struct OpenWindow<T: Window> {
    window: T,
    window_id: WindowId,
    pub(crate) redraw_status: RedrawStatus,
    scene: Arc<Scene>,
}

/// Allows requesting window refreshes outside of the event loop.
#[derive(Clone, Debug)]
pub struct RedrawRequester {
    event_sender: Sender<WindowEvent>,
}

impl RedrawRequester {
    /// Requests the window refresh itself. This will trigger [`Window::update`]
    /// before rendering.
    pub fn request_redraw(&self) {
        let _ = self.event_sender.send(WindowEvent::RedrawRequested);
    }

    /// Wakes the event loop, without necessarily redrawing.
    pub fn awaken(&self) {
        let _ = self.event_sender.send(WindowEvent::WakeUp);
    }
}

/// Tracks when a window should be redrawn. Allows for rendering a frame
/// immediately as well as scheduling a refresh in the future.
#[derive(Debug)]
pub struct RedrawStatus {
    next_redraw_target: RedrawTarget,
    needs_render: bool,
    needs_update: bool,
    event_sender: Sender<WindowEvent>,
}

impl RedrawStatus {
    /// Triggers a redraw as soon as possible. Any estimated frame instants will
    /// be ignored.
    pub fn set_needs_redraw(&mut self) {
        if !self.needs_render {
            self.needs_render = true;
            let _ = self.event_sender.send(WindowEvent::WakeUp);
        }
    }

    /// Triggers an update as soon as possible. Does not affect redrawing.
    pub fn set_needs_update(&mut self) {
        if !self.needs_render {
            self.needs_render = true;
            let _ = self.event_sender.send(WindowEvent::WakeUp);
        }
    }

    /// Estimates the next redraw instant by adding `duration` to
    /// `Instant::now()`. If this is later than the current estimate, it
    /// will be ignored.
    pub fn estimate_next_frame(&mut self, duration: Duration) {
        self.estimate_next_frame_instant(Instant::now().checked_add(duration).unwrap());
    }

    /// Estimates the next redraw instant. If `instant` is later than the
    /// current estimate, it will be ignored.
    pub fn estimate_next_frame_instant(&mut self, instant: Instant) {
        match self.next_redraw_target {
            RedrawTarget::Never | RedrawTarget::None => {
                self.next_redraw_target = RedrawTarget::Scheduled(instant);
            }
            RedrawTarget::Scheduled(existing_instant) => {
                if instant < existing_instant {
                    self.next_redraw_target = RedrawTarget::Scheduled(instant);
                }
            }
        }
    }

    /// Returns a redraw requester that can be used outside of the event loop.
    #[must_use]
    pub fn redraw_requester(&self) -> RedrawRequester {
        RedrawRequester {
            event_sender: self.event_sender.clone(),
        }
    }
}

impl<T: Window> OpenWindow<T> {
    pub(crate) fn new(
        window: T,
        window_id: WindowId,
        event_sender: Sender<WindowEvent>,
        scene: Scene,
    ) -> Self {
        Self {
            window,
            window_id,
            scene: Arc::new(scene),
            redraw_status: RedrawStatus {
                needs_render: true,
                needs_update: true,
                next_redraw_target: RedrawTarget::None,
                event_sender,
            },
        }
    }

    pub(crate) fn clear_redraw_target(&mut self) {
        self.redraw_status.needs_render = false;
        self.redraw_status.next_redraw_target = RedrawTarget::None;
    }

    pub(crate) fn set_has_updated(&mut self) {
        self.redraw_status.needs_update = false;
    }

    pub(crate) fn initialize_redraw_target(&mut self, target_fps: Option<u16>) {
        if let RedrawTarget::None = self.redraw_status.next_redraw_target {
            match target_fps {
                Some(fps) => {
                    self.redraw_status.next_redraw_target = RedrawTarget::Scheduled(
                        Instant::now()
                            .checked_add(Duration::from_secs_f32(1. / f32::from(fps)))
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

    pub(crate) fn can_wait_for_events(&self) -> bool {
        !self.should_redraw_now() && !self.redraw_status.needs_update
    }

    pub(crate) fn should_redraw_now(&self) -> bool {
        self.redraw_status.needs_render
            || match self.redraw_status.next_redraw_target {
                RedrawTarget::Never | RedrawTarget::None => false,
                RedrawTarget::Scheduled(scheduled_for) => scheduled_for < Instant::now(),
            }
    }

    pub(crate) fn request_close(&mut self) -> crate::Result<CloseResponse> {
        self.window.close_requested(WindowHandle(self.window_id))
    }

    pub(crate) fn process_input(&mut self, input: InputEvent) -> crate::Result<()> {
        self.window.process_input(
            input,
            &mut self.redraw_status,
            &Target::from(self.scene.clone()),
            WindowHandle(self.window_id),
        )
    }

    pub(crate) fn receive_character(&mut self, character: char) -> crate::Result<()> {
        self.window.receive_character(
            character,
            &mut self.redraw_status,
            &Target::from(self.scene.clone()),
            WindowHandle(self.window_id),
        )
    }

    pub(crate) fn initialize(&mut self) -> crate::Result<()> {
        self.window.initialize(
            &Target::from(self.scene.clone()),
            self.redraw_status.redraw_requester(),
            WindowHandle(self.window_id),
        )?;

        Ok(())
    }

    pub(crate) fn render(&mut self) -> crate::Result<()> {
        // Clear the redraw target first, so that if something inside of render
        // (or another thread) requests a redraw it will still be honored.
        self.clear_redraw_target();

        self.window.render(
            &Target {
                scene: self.scene.clone(),
                clip: None,
                offset: None,
            },
            &mut self.redraw_status,
            WindowHandle(self.window_id),
        )?;

        Ok(())
    }

    pub(crate) fn update(&mut self, target_fps: Option<u16>) -> crate::Result<()> {
        self.initialize_redraw_target(target_fps);
        self.set_has_updated();

        self.window.update(
            &Target {
                scene: self.scene.clone(),
                clip: None,
                offset: None,
            },
            &mut self.redraw_status,
            WindowHandle(self.window_id),
        )
    }

    pub(crate) fn additional_scale(&self) -> Scale<f32, Scaled, Points> {
        self.window.additional_scale()
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
pub enum RedrawTarget {
    None,
    Never,
    Scheduled(Instant),
}

impl Default for RedrawTarget {
    fn default() -> Self {
        Self::None
    }
}

pub enum UpdateSchedule {
    Now,
    Scheduled(Instant),
}

impl RedrawTarget {
    pub(crate) const fn next_update_instant(&self) -> Option<UpdateSchedule> {
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
