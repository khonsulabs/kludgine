use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use kludgine_core::{
    figures::{num_traits::One, Pixels, Point, Points},
    flume,
    math::{Scale, Scaled, Size},
    scene::Target,
    winit::{
        self,
        window::{Theme, WindowBuilder as WinitWindowBuilder, WindowId},
    },
};
use lazy_static::lazy_static;

use crate::{Error, Runtime};

/// Types for event handling.
pub mod event;
mod open;
mod runtime_window;

pub use open::{OpenWindow, RedrawRequester, RedrawStatus};
pub use runtime_window::{opened_first_window, RuntimeWindow, RuntimeWindowConfig, WindowHandle};
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
    /// Called when the window is being initilaized.
    fn initialize(
        &mut self,
        _scene: &Target,
        _redrawer: RedrawRequester,
        _window: WindowHandle,
    ) -> crate::Result<()>
    where
        Self: Sized,
    {
        Ok(())
    }

    /// The window was requested to be closed, most likely from the Close
    /// Button. Override this implementation if you want logic in place to
    /// prevent a window from closing.
    fn close_requested(&mut self, _window: WindowHandle) -> crate::Result<CloseResponse> {
        Ok(CloseResponse::Close)
    }

    /// The window has received an input event.
    fn process_input(
        &mut self,
        _input: InputEvent,
        _status: &mut RedrawStatus,
        _scene: &Target,
        _window: WindowHandle,
    ) -> crate::Result<()>
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
        _scene: &Target,
        _window: WindowHandle,
    ) -> crate::Result<()>
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

    /// Renders the contents of the window. Called whenever the operating system
    /// needs the window's contents to be redrawn or when [`RedrawStatus`]
    /// indicates a new frame should be rendered in [`Window::update()`].
    #[allow(unused_variables)]
    fn render(
        &mut self,
        scene: &Target,
        status: &mut RedrawStatus,
        _window: WindowHandle,
    ) -> crate::Result<()> {
        Ok(())
    }

    /// Called on a regular basis as events come in. Use `status` to indicate
    /// when a redraw should happen.
    #[allow(unused_variables)]
    fn update(
        &mut self,
        scene: &Target,
        status: &mut RedrawStatus,
        _window: WindowHandle,
    ) -> crate::Result<()>
    where
        Self: Sized,
    {
        Ok(())
    }

    /// Called prior to rendering to allow setting a scaling amount that
    /// operates on top of the automatic DPI scaling. This can be used to offer
    /// a zoom setting to end-users.
    fn additional_scale(&self) -> Scale<f32, Scaled, Points> {
        Scale::one()
    }
}

/// Defines initial window properties.
pub trait WindowCreator: Window {
    /// Returns a [`WindowBuilder`] for this window.
    #[must_use]
    fn get_window_builder(&self) -> WindowBuilder {
        WindowBuilder::default()
            .with_title(self.window_title())
            .with_initial_system_theme(self.initial_system_theme())
            .with_size(self.initial_size())
            .with_resizable(self.resizable())
            .with_maximized(self.maximized())
            .with_visible(self.visible())
            .with_transparent(self.transparent())
            .with_decorations(self.decorations())
            .with_always_on_top(self.always_on_top())
    }

    /// The initial title of the window.
    #[must_use]
    fn window_title(&self) -> String {
        "Kludgine".to_owned()
    }

    /// The initial size of the window.
    #[must_use]
    fn initial_size(&self) -> Size<u32, Pixels> {
        Size::new(1024, 768)
    }

    /// Whether the window should be resizable or not.
    #[must_use]
    fn resizable(&self) -> bool {
        true
    }

    /// Whether the window should be maximized or not.
    #[must_use]
    fn maximized(&self) -> bool {
        false
    }

    /// Whether the window should be visible or not.
    #[must_use]
    fn visible(&self) -> bool {
        true
    }

    /// Whether the window should be transparent or not.
    #[must_use]
    fn transparent(&self) -> bool {
        false
    }

    /// Whether the window should be drawn with decorations or not.
    #[must_use]
    fn decorations(&self) -> bool {
        true
    }

    /// Whether the window should always be on top or not.
    #[must_use]
    fn always_on_top(&self) -> bool {
        false
    }

    /// The default [`Theme`] for the [`Window`] if `winit` is unable to
    /// determine the system theme.
    #[must_use]
    fn initial_system_theme(&self) -> Theme {
        Theme::Light
    }
}

/// A builder for a [`Window`].
#[derive(Default)]
pub struct WindowBuilder {
    title: Option<String>,
    position: Option<Point<i32, Pixels>>,
    size: Option<Size<u32, Pixels>>,
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
    /// Builder-style function. Sets `title` and returns self.
    #[must_use]
    pub fn with_title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Builder-style function. Sets `size` and returns self.
    #[must_use]
    pub const fn with_position(mut self, position: Point<i32, Pixels>) -> Self {
        self.position = Some(position);
        self
    }

    /// Builder-style function. Sets `size` and returns self.
    #[must_use]
    pub const fn with_size(mut self, size: Size<u32, Pixels>) -> Self {
        self.size = Some(size);
        self
    }

    /// Builder-style function. Sets `resizable` and returns self.
    #[must_use]
    pub const fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = Some(resizable);
        self
    }

    /// Builder-style function. Sets `maximized` and returns self.
    #[must_use]
    pub const fn with_maximized(mut self, maximized: bool) -> Self {
        self.maximized = Some(maximized);
        self
    }

    /// Builder-style function. Sets `visible` and returns self.
    #[must_use]
    pub const fn with_visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    /// Builder-style function. Sets `transparent` and returns self.
    #[must_use]
    pub const fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = Some(transparent);
        self
    }

    /// Builder-style function. Sets `decorations` and returns self.
    #[must_use]
    pub const fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = Some(decorations);
        self
    }

    /// Builder-style function. Sets `alawys_on_top` and returns self.
    #[must_use]
    pub const fn with_always_on_top(mut self, always_on_top: bool) -> Self {
        self.always_on_top = Some(always_on_top);
        self
    }

    /// Builder-style function. Sets `icon` and returns self.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // unsupported
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Builder-style function. Sets `initial_system_theme` and returns self.
    #[must_use]
    pub const fn with_initial_system_theme(mut self, system_theme: Theme) -> Self {
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
        if let Some(position) = wb.position {
            builder = builder.with_position(winit::dpi::Position::Physical(
                winit::dpi::PhysicalPosition {
                    x: position.x,
                    y: position.y,
                },
            ));
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

/// A window that can be opened.
#[cfg(feature = "multiwindow")]
pub trait OpenableWindow {
    /// Opens `self` as a [`Window`].
    fn open(self);
}

#[cfg(feature = "multiwindow")]
impl<T> OpenableWindow for T
where
    T: Window + WindowCreator,
{
    fn open(self) {
        crate::runtime::Runtime::open_window(self.get_window_builder(), self);
    }
}

lazy_static! {
    static ref WINDOW_CHANNELS: Arc<Mutex<HashMap<WindowId, flume::Sender<WindowMessage>>>> =
        Arc::default();
}

pub enum WindowMessage {
    Close,
    RequestClose,
    SetAdditionalScale(Scale<f32, Scaled, Points>),
}

impl WindowMessage {
    pub fn send_to(self, id: WindowId) -> crate::Result<()> {
        let sender = {
            let mut channels = WINDOW_CHANNELS.lock().unwrap();
            if let Some(sender) = channels.get_mut(&id) {
                sender.clone()
            } else {
                return Err(Error::InternalWindowMessageSend(
                    "Channel not found for id".to_owned(),
                ));
            }
        };

        sender.send(self).unwrap_or_default();
        Runtime::try_process_window_events(None);
        Ok(())
    }
}
