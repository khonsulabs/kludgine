use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use easygpu::prelude::*;
use lazy_static::lazy_static;
use winit::window::{Theme, WindowBuilder as WinitWindowBuilder, WindowId};

use crate::{
    math::{Scaled, Size},
    scene::Target,
    KludgineError, KludgineResult,
};

pub mod event;
pub(crate) mod open;
mod runtime_window;

pub use open::{OpenWindow, RedrawStatus};
pub(crate) use runtime_window::{opened_first_window, RuntimeWindow, RuntimeWindowConfig};
pub use winit::window::Icon;

use self::event::InputEvent;

/// How to react to a request to close a window
pub enum CloseResponse {
    /// Window should remain open
    RemainOpen,
    /// Window should close
    Close,
}

/// Trait to implement a Window
pub trait Window: Send + Sync + 'static {
    fn initialize(&mut self, _scene: &Target) -> KludgineResult<()>
    where
        Self: Sized,
    {
        Ok(())
    }
    /// The window was requested to be closed, most likely from the Close
    /// Button. Override this implementation if you want logic in place to
    /// prevent a window from closing.
    fn close_requested(&mut self) -> KludgineResult<CloseResponse> {
        Ok(CloseResponse::Close)
    }

    /// The window has received an input event.
    fn process_input(
        &mut self,
        _input: InputEvent,
        _status: &mut RedrawStatus,
    ) -> KludgineResult<()>
    where
        Self: Sized,
    {
        Ok(())
    }

    /// A text input was received.
    fn receive_character(
        &mut self,
        _character: char,
        _status: &mut RedrawStatus,
    ) -> KludgineResult<()>
    where
        Self: Sized,
    {
        Ok(())
    }

    /// Specify a target frames per second, which will force your window
    /// to redraw at this rate. If None is returned, the Window will only
    /// redraw when requested via methods on Context.
    fn target_fps(&self) -> Option<u16> {
        None
    }

    fn render(&mut self, _scene: &Target) -> KludgineResult<()> {
        Ok(())
    }

    fn update(&mut self, _scene: &Target, _status: &mut RedrawStatus) -> KludgineResult<()>
    where
        Self: Sized,
    {
        Ok(())
    }
}

pub trait WindowCreator: Window {
    fn get_window_builder() -> WindowBuilder {
        WindowBuilder::default()
            .with_title(Self::window_title())
            .with_initial_system_theme(Self::initial_system_theme())
            .with_size(Self::initial_size())
            .with_resizable(Self::resizable())
            .with_maximized(Self::maximized())
            .with_visible(Self::visible())
            .with_transparent(Self::transparent())
            .with_decorations(Self::decorations())
            .with_always_on_top(Self::always_on_top())
    }

    fn window_title() -> String {
        "Kludgine".to_owned()
    }

    fn initial_size() -> Size<u32, Scaled> {
        Size::new(1024, 768)
    }

    fn resizable() -> bool {
        true
    }

    fn maximized() -> bool {
        false
    }

    fn visible() -> bool {
        true
    }

    fn transparent() -> bool {
        false
    }

    fn decorations() -> bool {
        true
    }

    fn always_on_top() -> bool {
        false
    }

    fn initial_system_theme() -> Theme {
        Theme::Light
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
    pub(crate) initial_system_theme: Option<Theme>,
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

    pub fn with_initial_system_theme(mut self, system_theme: Theme) -> Self {
        self.initial_system_theme = Some(system_theme);
        self
    }
}

impl From<WindowBuilder> for WinitWindowBuilder {
    fn from(wb: WindowBuilder) -> Self {
        let mut builder = Self::new();
        if let Some(title) = wb.title {
            builder = builder.with_title(title);
        }
        if let Some(size) = wb.size {
            builder =
                builder.with_inner_size(winit::dpi::Size::Physical(winit::dpi::PhysicalSize {
                    width: size.width,
                    height: size.height,
                }));
        }
        if let Some(resizable) = wb.resizable {
            builder = builder.with_resizable(resizable);
        }
        if let Some(maximized) = wb.maximized {
            builder = builder.with_maximized(maximized);
        }
        if let Some(visible) = wb.visible {
            builder = builder.with_visible(visible);
        }
        if let Some(transparent) = wb.transparent {
            builder = builder.with_transparent(transparent);
        }
        if let Some(decorations) = wb.decorations {
            builder = builder.with_decorations(decorations);
        }
        if let Some(always_on_top) = wb.always_on_top {
            builder = builder.with_always_on_top(always_on_top);
        }

        builder = builder.with_window_icon(wb.icon);

        builder
    }
}

#[cfg(feature = "multiwindow")]
pub trait OpenableWindow {
    fn open(window: Self);
}

#[cfg(feature = "multiwindow")]
impl<T> OpenableWindow for T
where
    T: Window + WindowCreator,
{
    fn open(window: Self) {
        crate::runtime::Runtime::open_window(Self::get_window_builder(), window)
    }
}

lazy_static! {
    static ref WINDOW_CHANNELS: Arc<Mutex<HashMap<WindowId, flume::Sender<WindowMessage>>>> =
        Arc::default();
}
pub(crate) enum WindowMessage {
    Close,
}

impl WindowMessage {
    pub fn send_to(self, id: WindowId) -> KludgineResult<()> {
        let sender = {
            let mut channels = WINDOW_CHANNELS.lock().unwrap();
            if let Some(sender) = channels.get_mut(&id) {
                sender.clone()
            } else {
                return Err(KludgineError::InternalWindowMessageSendError(
                    "Channel not found for id".to_owned(),
                ));
            }
        };

        sender.send(self).unwrap_or_default();
        Ok(())
    }
}
