use crate::internal_prelude::*;
use crate::scene2d::Scene2d;
use glutin::event::{DeviceId, ElementState, TouchPhase, VirtualKeyCode};

pub enum CloseResponse {
    RemainOpen,
    Close,
}

#[derive(Clone)]
pub struct InputEvent {
    pub device_id: DeviceId,
    pub event: Event,
}

#[derive(Clone)]
pub enum Event {
    Keyboard {
        key: Option<VirtualKeyCode>,
        state: ElementState,
    },
    MouseButton {
        button: MouseButton,
        state: ElementState,
    },
    MouseMoved {
        position: Option<Point2d>,
    },
    MouseWheel {
        delta: MouseScrollDelta,
        touch_phase: TouchPhase,
    },
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

    async fn process_input(&mut self, _event: InputEvent) -> KludgineResult<()> {
        Ok(())
    }
}
