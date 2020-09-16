use crate::{
    event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, TouchPhase, VirtualKeyCode},
    math::{Point, Scaled, ScreenScale, Size},
    runtime::Runtime,
    ui::InteractiveComponent,
    Handle, KludgineError, KludgineResult,
};
use async_trait::async_trait;

use crossbeam::sync::ShardedLock;
use lazy_static::lazy_static;
use rgx::core::*;

use std::collections::HashMap;
use winit::window::{WindowBuilder as WinitWindowBuilder, WindowId};

pub(crate) mod frame;
mod renderer;
mod runtime_window;
pub(crate) use runtime_window::RuntimeWindow;

pub use winit::window::Icon;

/// How to react to a request to close a window
pub enum CloseResponse {
    /// Window should remain open
    RemainOpen,
    /// Window should close
    Close,
}

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

/// Trait to implement a Window
#[async_trait]
pub trait Window: InteractiveComponent + Send + Sync + 'static {
    /// The window was requested to be closed, most likely from the Close Button. Override
    /// this implementation if you want logic in place to prevent a window from closing.
    async fn close_requested(&self) -> KludgineResult<CloseResponse> {
        Ok(CloseResponse::Close)
    }

    /// Specify a target frames per second, which will force your window
    /// to redraw at this rate. If None is returned, the Window will only
    /// redraw when requested via methods on Context.
    fn target_fps(&self) -> Option<u16> {
        None
    }
}

pub trait WindowCreator<T>: Window {
    fn get_window_builder() -> WindowBuilder {
        WindowBuilder::default().with_title(Self::window_title())
    }

    fn window_title() -> String {
        "Kludgine".to_owned()
    }
}

#[derive(Default)]
pub struct WindowBuilder {
    title: Option<String>,
    size: Option<Size<u32, Scaled>>,
    resizable: Option<bool>,
    maximized: Option<bool>,
    visible: Option<bool>,
    transparent: Option<bool>,
    decorations: Option<bool>,
    always_on_top: Option<bool>,
    icon: Option<winit::window::Icon>,
}

impl WindowBuilder {
    pub fn with_title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_size(mut self, size: Size<u32, Scaled>) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = Some(resizable);
        self
    }

    pub fn with_maximized(mut self, maximized: bool) -> Self {
        self.maximized = Some(maximized);
        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = Some(transparent);
        self
    }

    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = Some(decorations);
        self
    }

    pub fn with_always_on_top(mut self, always_on_top: bool) -> Self {
        self.always_on_top = Some(always_on_top);
        self
    }

    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl Into<WinitWindowBuilder> for WindowBuilder {
    fn into(self) -> WinitWindowBuilder {
        let mut builder = WinitWindowBuilder::new();
        if let Some(title) = self.title {
            builder = builder.with_title(title);
        }
        if let Some(size) = self.size {
            builder =
                builder.with_inner_size(winit::dpi::Size::Physical(winit::dpi::PhysicalSize {
                    width: size.width,
                    height: size.height,
                }));
        }
        if let Some(resizable) = self.resizable {
            builder = builder.with_resizable(resizable);
        }
        if let Some(maximized) = self.maximized {
            builder = builder.with_maximized(maximized);
        }
        if let Some(visible) = self.visible {
            builder = builder.with_visible(visible);
        }
        if let Some(transparent) = self.transparent {
            builder = builder.with_transparent(transparent);
        }
        if let Some(decorations) = self.decorations {
            builder = builder.with_decorations(decorations);
        }
        if let Some(always_on_top) = self.always_on_top {
            builder = builder.with_always_on_top(always_on_top);
        }

        builder = builder.with_window_icon(self.icon);

        builder
    }
}

#[async_trait]
pub trait OpenableWindow {
    async fn open(window: Self);
}

#[async_trait]
impl<T> OpenableWindow for T
where
    T: Window + WindowCreator<T>,
{
    async fn open(window: Self) {
        Runtime::open_window(Self::get_window_builder(), window).await
    }
}

lazy_static! {
    static ref WINDOW_CHANNELS: Handle<HashMap<WindowId, async_channel::Sender<WindowMessage>>> =
        Handle::new(HashMap::new());
}

lazy_static! {
    static ref WINDOWS: ShardedLock<HashMap<WindowId, RuntimeWindow>> =
        ShardedLock::new(HashMap::new());
}

pub(crate) enum WindowMessage {
    Close,
}

impl WindowMessage {
    pub async fn send_to(self, id: WindowId) -> KludgineResult<()> {
        let sender = {
            let mut channels = WINDOW_CHANNELS.write().await;
            if let Some(sender) = channels.get_mut(&id) {
                sender.clone()
            } else {
                return Err(KludgineError::InternalWindowMessageSendError(
                    "Channel not found for id".to_owned(),
                ));
            }
        };

        sender.send(self).await.unwrap_or_default();
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum WindowEvent {
    CloseRequested,
    Resize {
        size: Size,
        scale_factor: ScreenScale,
    },
    Input(InputEvent),
    RedrawRequested,
}
