use crate::math::{Point, Scaled, ScreenScale, Size};
pub use winit::event::{
    DeviceId, ElementState, MouseButton, MouseScrollDelta, ScanCode, TouchPhase, VirtualKeyCode,
};
use winit::window::Theme;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum EventStatus {
    Ignored,
    Processed,
}

impl Default for EventStatus {
    fn default() -> Self {
        EventStatus::Ignored
    }
}

impl EventStatus {
    pub fn update_with(&mut self, other: Self) {
        *self = if self == &EventStatus::Processed || other == EventStatus::Processed {
            EventStatus::Processed
        } else {
            EventStatus::Ignored
        };
    }
}

/// An Event from a device
#[derive(Copy, Clone, Debug)]
pub struct InputEvent {
    /// The device that triggered this event
    pub device_id: DeviceId,
    /// The event that was triggered
    pub event: Event,
}

/// An input Event
#[derive(Copy, Clone, Debug)]
pub enum Event {
    /// A keyboard event
    Keyboard {
        scancode: ScanCode,
        key: Option<VirtualKeyCode>,
        state: ElementState,
    },
    /// A mouse button event
    MouseButton {
        button: MouseButton,
        state: ElementState,
    },
    /// Mouse cursor event
    MouseMoved {
        position: Option<Point<f32, Scaled>>,
    },
    /// Mouse wheel event
    MouseWheel {
        delta: MouseScrollDelta,
        touch_phase: TouchPhase,
    },
}

#[derive(Debug)]
pub(crate) enum WindowEvent {
    WakeUp,
    CloseRequested,
    Resize {
        size: Size,
        scale_factor: ScreenScale,
    },
    Input(InputEvent),
    ReceiveCharacter(char),
    RedrawRequested,
    SystemThemeChanged(Theme),
}
