use kludgine_core::figures::{Pixels, Points};
use kludgine_core::math::{Point, Scale, Scaled, Size};
pub use kludgine_core::winit::event::{
    DeviceId, ElementState, MouseButton, MouseScrollDelta, ScanCode, TouchPhase, VirtualKeyCode,
};
use kludgine_core::winit::window::Theme;

/// Whether an event has been processed or ignored.
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum EventStatus {
    /// The event was not handled.
    Ignored,
    /// The event was handled and should not be processed any further.
    Processed,
}

impl Default for EventStatus {
    fn default() -> Self {
        Self::Ignored
    }
}

impl EventStatus {
    /// Updates `self` such that if either `self` or `other` are `Processed`,
    /// `self` will be proecssed.
    pub fn update_with(&mut self, other: Self) {
        if self != &other {
            *self = if self == &Self::Processed || other == Self::Processed {
                Self::Processed
            } else {
                Self::Ignored
            };
        }
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
        /// The hardware-dependent scan code.
        scancode: ScanCode,
        /// Contains a [`VirtualKeyCode`] if `scancode` was interpetted as a
        /// known key.
        key: Option<VirtualKeyCode>,
        /// Indicates pressed or released/
        state: ElementState,
    },
    /// A mouse button event
    MouseButton {
        /// The button tha triggered this event.
        button: MouseButton,
        /// Indicates pressed or released/
        state: ElementState,
    },
    /// Mouse cursor event
    MouseMoved {
        /// The location within the window of the cursor. Will be invoked with
        /// `None` when the cursor leaves the window.
        position: Option<Point<f32, Scaled>>,
    },
    /// Mouse wheel event
    MouseWheel {
        /// The scroll amount.
        delta: MouseScrollDelta,
        /// If this event was caused by touch events, the phase of the touch.
        touch_phase: TouchPhase,
    },
}

#[derive(Debug)]
pub(crate) enum WindowEvent {
    WakeUp,
    CloseRequested,
    Resize {
        size: Size<u32, Pixels>,
        scale_factor: Scale<f32, Points, Pixels>,
    },
    Input(InputEvent),
    ReceiveCharacter(char),
    RedrawRequested,
    SystemThemeChanged(Theme),
}
