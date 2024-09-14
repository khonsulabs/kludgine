use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use appit::winit::dpi::{PhysicalPosition, PhysicalSize};
use appit::winit::error::{EventLoopError, OsError};
use appit::winit::event::{
    AxisId, DeviceId, ElementState, Ime, KeyEvent, Modifiers, MouseButton, MouseScrollDelta, Touch,
    TouchPhase,
};
use appit::winit::event_loop::OwnedDisplayHandle;
use appit::winit::keyboard::PhysicalKey;
use appit::winit::monitor::{MonitorHandle, VideoModeHandle};
use appit::winit::window::{ImePurpose, Theme, WindowId};
pub use appit::{winit, Application, AsApplication, Message, WindowAttributes};
use appit::{RunningWindow, WindowBehavior as _};
use figures::units::{Px, UPx};
use figures::{Fraction, IntoSigned, Point, Rect, Size};
use intentional::{Assert, Cast};

use crate::drawing::{Drawing, Renderer};
use crate::{Color, Graphics, Kludgine, RenderingGraphics};

/// A `Kludgine` application that enables opening multiple windows.
pub struct PendingApp<WindowEvent = ()>(appit::PendingApp<AppEvent<WindowEvent>>)
where
    AppEvent<WindowEvent>: Message<Window = WindowEvent, Response = AppResponse>;

impl<WindowEvent> Default for PendingApp<WindowEvent>
where
    AppEvent<WindowEvent>: Message<Window = WindowEvent, Response = AppResponse>,
    WindowEvent: Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<WindowEvent> AsApplication<AppEvent<WindowEvent>> for PendingApp<WindowEvent>
where
    AppEvent<WindowEvent>: Message<Window = WindowEvent, Response = AppResponse>,
{
    fn as_application(&self) -> &dyn Application<AppEvent<WindowEvent>>
    where
        AppEvent<WindowEvent>: Message,
    {
        &self.0
    }

    fn as_application_mut(&mut self) -> &mut dyn Application<AppEvent<WindowEvent>>
    where
        AppEvent<WindowEvent>: Message,
    {
        &mut self.0
    }
}

impl<WindowEvent> PendingApp<WindowEvent>
where
    AppEvent<WindowEvent>: Message<Window = WindowEvent, Response = AppResponse>,
    WindowEvent: Send + 'static,
{
    /// Creates a new Kludgine application.
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // The panics are in a closure that happens only after the app is running.
    pub fn new() -> Self {
        Self(appit::PendingApp::new_with_event_callback(
            |request: AppEvent<WindowEvent>,
             windows: appit::ExecutingApp<'_, AppEvent<WindowEvent>>| {
                match request.0 {
                    AppEventKind::CreateSurface(request) => {
                        let window = windows.get(request.window).expect("window not found");
                        AppResponse(AppResponseKind::Surface(
                            request
                                .wgpu
                                .create_surface(window)
                                .expect("error creating surface"),
                        ))
                    }
                    AppEventKind::ListMonitors => {
                        AppResponse(AppResponseKind::Monitors(Monitors {
                            primary: windows.primary_monitor().map(Monitor),
                            available: windows
                                .available_monitors()
                                .into_iter()
                                .map(Monitor)
                                .collect(),
                        }))
                    }
                }
            },
        ))
    }

    /// Returns a handle to the application that will be run.
    #[must_use]
    pub fn as_app(&self) -> App<WindowEvent> {
        App(self.0.app())
    }

    /// Executes `on_startup` once the app event loop has started.
    ///
    /// This is useful because some information provided by winit is only
    /// available after the event loop has started. For example, to enter an
    /// exclusive full screen mode, monitor information must be accessed which
    /// requires the event loop to have been started.
    pub fn on_startup<F>(&mut self, on_startup: F)
    where
        F: FnOnce(ExecutingApp<'_, WindowEvent>) + Send + 'static,
    {
        self.0
            .on_startup(|app: appit::ExecutingApp<'_, AppEvent<WindowEvent>>| {
                on_startup(ExecutingApp(app));
            });
    }

    /// Begins running the application.
    ///
    /// On some platforms, this function may never return. If it does return, it
    /// is after the application has been shut down.
    ///
    /// # Errors
    ///
    /// Returns an [`EventLoopError`] upon the loop exiting due to an error. See
    /// [`EventLoop::run`](winit::event_loop::EventLoop::run) for more
    /// information.
    pub fn run(self) -> Result<(), EventLoopError> {
        self.0.run()
    }
}

/// A reference to an executing application and its event loop.
pub struct ExecutingApp<'a, WindowEvent = ()>(appit::ExecutingApp<'a, AppEvent<WindowEvent>>)
where
    AppEvent<WindowEvent>: Message;

impl<WindowEvent> ExecutingApp<'_, WindowEvent>
where
    AppEvent<WindowEvent>: Message,
{
    /// Returns the list of available monitors.
    ///
    /// This function will return an empty `Vec` if invoked before the
    /// application has begun executing. This can occur if an app message is
    /// sent before a `PendingApp` is run.
    #[must_use]
    pub fn available_monitors(&self) -> Vec<Monitor> {
        self.0
            .available_monitors()
            .into_iter()
            .map(Monitor)
            .collect()
    }

    /// Returns a handle to the primary monitor.
    ///
    /// This function will return None if:
    ///
    /// - The application hasn't begun executing.
    /// - The platform does not support determining a primary monitor.
    #[must_use]
    pub fn primary_monitor(&self) -> Option<Monitor> {
        self.0.primary_monitor().map(Monitor)
    }

    /// Returns a handle to the underlying display.
    #[must_use]
    pub fn owned_display_handle(&self) -> OwnedDisplayHandle {
        self.0.owned_display_handle()
    }
}

/// A handle to a running Kludgine application.
pub struct App<WindowEvent = ()>(appit::App<AppEvent<WindowEvent>>)
where
    WindowEvent: Send + 'static;

impl<WindowEvent> App<WindowEvent>
where
    WindowEvent: Send + 'static,
{
    /// Returns a snapshot of information about the monitors on this device.
    ///
    /// This function will return None if the application has not started
    /// running yet or the application has shut down.
    pub fn monitors(&self) -> Option<Monitors> {
        self.0
            .send(AppEvent(AppEventKind::ListMonitors))
            .map(AppResponse::expect_monitors)
            .filter(|monitors| !monitors.available.is_empty())
    }

    /// Creates a guard that prevents this app from shutting down.
    ///
    /// If the app is not currently running, this function returns None.
    ///
    /// Once a guard is allocated the app will not be closed automatically when
    /// the final window is closed. If the final shutdown guard is dropped while
    /// no windows are open, the app will be closed.
    #[allow(clippy::must_use_candidate)]
    pub fn prevent_shutdown(&self) -> Option<ShutdownGuard<WindowEvent>> {
        self.0.prevent_shutdown()
    }
}

/// A guard preventing an [`App`] from shutting down.
pub type ShutdownGuard<WindowEvent> = appit::ShutdownGuard<AppEvent<WindowEvent>>;

impl<WindowEvent> Clone for App<WindowEvent>
where
    WindowEvent: Send + 'static,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<WindowEvent> AsApplication<AppEvent<WindowEvent>> for App<WindowEvent>
where
    WindowEvent: Send + 'static,
{
    fn as_application(&self) -> &dyn Application<AppEvent<WindowEvent>>
    where
        AppEvent<WindowEvent>: Message,
    {
        self.0.as_application()
    }

    fn as_application_mut(&mut self) -> &mut dyn Application<AppEvent<WindowEvent>>
    where
        AppEvent<WindowEvent>: Message,
    {
        self.0.as_application_mut()
    }
}

/// An open window.
pub struct Window<'window, WindowEvent = ()>
where
    WindowEvent: Send + 'static,
{
    window: &'window mut RunningWindow<AppEvent<WindowEvent>>,
    elapsed: Duration,
    last_frame_rendered_in: Duration,
}

impl<'window, WindowEvent> Window<'window, WindowEvent>
where
    WindowEvent: Send + 'static,
{
    fn new(
        window: &'window mut RunningWindow<AppEvent<WindowEvent>>,
        elapsed: Duration,
        last_frame_rendered_in: Duration,
    ) -> Self {
        Self {
            window,
            elapsed,
            last_frame_rendered_in,
        }
    }

    /// Returns a handle to this window, which can be used to send
    /// `WindowEvent`s to it.
    #[must_use]
    pub fn handle(&self) -> WindowHandle<WindowEvent> {
        WindowHandle(self.window.handle())
    }

    /// Returns a handle to the application.
    ///
    /// This is useful for opening additional windows in a multi-window
    /// application.
    #[must_use]
    pub fn app(&self) -> App<WindowEvent> {
        App(self.window.app())
    }

    /// Returns a reference to the underlying winit window.
    #[must_use]
    pub fn winit(&self) -> &winit::window::Window {
        self.window.winit()
    }

    /// Closes this window as soon as control returns to `Kludgine`.
    pub fn close(&mut self) {
        self.window.close();
    }

    /// Returns the current inner position of the window.
    #[must_use]
    pub fn inner_position(&self) -> Point<Px> {
        self.window.inner_position().into()
    }

    /// Returns the current outer position of the window.
    #[must_use]
    pub fn outer_position(&self) -> Point<Px> {
        self.window.outer_position().into()
    }

    /// Sets the current outer position of the window.
    pub fn set_outer_position(&self, position: Point<Px>) {
        self.window.set_outer_position(position.into());
    }

    /// Returns the current DPI scale of the window.
    #[must_use]
    pub fn scale(&self) -> f64 {
        self.window.scale()
    }

    /// Returns the inner size of the window.
    #[must_use]
    pub fn inner_size(&self) -> Size<UPx> {
        self.window.inner_size().into()
    }

    /// Sets the inner size of the window.
    pub fn set_inner_size(&self, inner_size: Size<UPx>) {
        self.window.set_inner_size(inner_size.into());
    }

    /// Returns the size of the window, including decorations.
    #[must_use]
    pub fn outer_size(&self) -> Size<UPx> {
        self.window.outer_size().into()
    }

    /// Returns true if the window is currently focused for keyboard input.
    #[must_use]
    pub const fn focused(&self) -> bool {
        self.window.focused()
    }

    /// Returns the current user interface theme for the window.
    #[must_use]
    pub const fn theme(&self) -> Theme {
        self.window.theme()
    }

    /// Returns true if the window is currenly not visible because it is
    /// completely hidden behind other windows, offcreen, or minimized.
    #[must_use]
    pub const fn occluded(&self) -> bool {
        self.window.occluded()
    }

    /// Returns the current title of the window.
    #[must_use]
    pub fn title(&self) -> String {
        self.window.title()
    }

    /// Sets the title of the window.
    pub fn set_title(&mut self, new_title: &str) {
        self.window.set_title(new_title);
    }

    /// Sets whether IME input is allowed on the window.
    pub fn set_ime_allowed(&self, allowed: bool) {
        self.window.winit().set_ime_allowed(allowed);
    }

    /// Sets the IME purpose.
    pub fn set_ime_purpose(&self, purpose: ImePurpose) {
        self.window.winit().set_ime_purpose(purpose);
    }

    /// Sets the cursor area for IME input suggestions.
    pub fn set_ime_cursor_area(&self, area: Rect<UPx>) {
        self.window.winit().set_ime_cursor_area(
            PhysicalPosition::<u32>::new(area.origin.x.into(), area.origin.y.into()),
            PhysicalSize::<u32>::new(area.size.width.into(), area.size.height.into()),
        );
    }

    /// Sets the window to redraw after a `duration`.
    ///
    /// If the window is already set to redraw sooner, this function does
    /// nothing.
    pub fn redraw_in(&mut self, duration: Duration) {
        self.window.redraw_in(duration);
    }

    /// Sets the window to redraw at the provided time.
    ///
    /// If the window is already set to redraw sooner, this function does
    /// nothing.
    pub fn redraw_at(&mut self, time: Instant) {
        self.window.redraw_at(time);
    }

    /// Sets the window to redraw as soon as it can.
    pub fn set_needs_redraw(&mut self) {
        self.window.set_needs_redraw();
    }

    /// Returns the duration that has elapsed since the last frame start and the
    /// current frame start.
    ///
    /// This value is calculated once per frame and will not update between
    /// calls within the same event.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Returns the duration taken between when the last frame's redraw started
    /// and when the surface's frame was presented.
    #[must_use]
    pub const fn last_frame_rendered_in(&self) -> Duration {
        self.last_frame_rendered_in
    }

    /// Returns the position of the mouse cursor within this window, if the
    /// cursor is currently above the window.
    pub fn cursor_position(&self) -> Option<Point<Px>> {
        self.window.cursor_position().map(Point::from)
    }

    /// Returns true if the given button is currently pressed.
    #[must_use]
    pub fn mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.window.mouse_button_pressed(&button)
    }

    /// Returns true if the given virtual key code is currently pressed.
    #[must_use]
    pub fn key_pressed(&self, key: impl Into<PhysicalKey>) -> bool {
        self.window.key_pressed(&key.into())
    }

    /// Returns currently active modifiers.
    #[must_use]
    pub fn modifiers(&self) -> Modifiers {
        self.window.modifiers()
    }

    /// Sets the window's minimum inner size.
    pub fn set_min_inner_size(&self, min_size: Option<Size<UPx>>) {
        self.window.set_min_inner_size(min_size.map(Into::into));
    }

    /// Sets the window's maximum inner size.
    pub fn set_max_inner_size(&self, max_size: Option<Size<UPx>>) {
        self.window.set_max_inner_size(max_size.map(Into::into));
    }
}

/// The behavior of a window.
pub trait WindowBehavior<WindowEvent = ()>: Sized + 'static
where
    WindowEvent: Send + 'static,
{
    /// The type of value provided during [`initialize()`](Self::initialize).
    ///
    /// In Kludgine, each window runs in its own thread. Kludgine allows does
    /// not require `WindowBehavior` implementors to implement `Send`, but it
    /// can be useful to receive input from the thread that is opening the
    /// window. This is where `Context` is useful: it must implement `Send`,
    /// allowing some data to still be passed when opening the window.
    type Context: Send + 'static;

    /// Initialize a new instance from the provided context.
    fn initialize(
        window: Window<'_, WindowEvent>,
        graphics: &mut Graphics<'_>,
        context: Self::Context,
    ) -> Self;

    /// Returns the window attributes to use when creating the window.
    #[must_use]
    #[allow(unused_variables)]
    fn initial_window_attributes(context: &Self::Context) -> WindowAttributes {
        WindowAttributes::default()
    }

    /// Returns the power preference to initialize `wgpu` with.
    #[must_use]
    #[allow(unused_variables)]
    fn power_preference(context: &Self::Context) -> wgpu::PowerPreference {
        wgpu::PowerPreference::default()
    }

    /// Returns the memory hints to initialize `wgpu` with.
    #[must_use]
    #[allow(unused_variables)]
    fn memory_hints(context: &Self::Context) -> wgpu::MemoryHints {
        wgpu::MemoryHints::default()
    }

    /// Returns the limits to apply for the `wgpu` instance.
    #[must_use]
    #[allow(unused_variables)]
    fn limits(adapter_limits: wgpu::Limits, context: &Self::Context) -> wgpu::Limits {
        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter_limits)
    }

    /// Returns the number of multisamples to perform when rendering this
    /// window.
    ///
    /// When 1 is returned, multisampling will be fully disabled.
    #[must_use]
    #[allow(unused_variables)]
    fn multisample_count(context: &Self::Context) -> NonZeroU32 {
        NonZeroU32::new(4).assert("4 is less than u32::MAX")
    }

    /// Executed once after the window has been fully initialized.
    #[allow(unused_variables)]
    fn initialized(&mut self, window: Window<'_, WindowEvent>, kludgine: &mut Kludgine) {}

    /// Prepare the window to render.
    ///
    /// This is called directly before [`render()`](Self::render()) and is a
    /// perfect place to update any prepared graphics as needed.
    #[allow(unused_variables)]
    fn prepare(&mut self, window: Window<'_, WindowEvent>, graphics: &mut Graphics<'_>) {}

    /// Render the contents of the window.
    // TODO refactor away from bool return.
    fn render<'pass>(
        &'pass mut self,
        window: Window<'_, WindowEvent>,
        graphics: &mut RenderingGraphics<'_, 'pass>,
    ) -> bool;

    /// Returns the swap chain present mode to use for this window.
    #[must_use]
    fn present_mode(&self) -> wgpu::PresentMode {
        wgpu::PresentMode::AutoVsync
    }

    /// Returns the color to clear the window with. If None is returned, the
    /// window will not be cleared between redraws.
    ///
    /// The default implementation returns `Some(Color::BLACK)`.
    #[must_use]
    fn clear_color(&self) -> Option<Color> {
        Some(Color::BLACK)
    }

    /// Returns the composite alpha mode to use for rendering the wgpu surface
    /// on the window.
    ///
    /// `supported_modes` contains the list of detected alpha modes supported by
    /// the surface.
    #[must_use]
    fn composite_alpha_mode(
        &self,
        supported_modes: &[wgpu::CompositeAlphaMode],
    ) -> wgpu::CompositeAlphaMode {
        supported_modes[0]
    }

    /// Launches a Kludgine app using this window as the primary window.
    ///
    /// # Panics
    ///
    /// On many platforms, it is a requirement that this function only be called
    /// from the thread that is executing the program's `main()` function.
    /// `wgpu` may panic when creating a surface if this function is not called
    /// from the correct thread.
    ///
    /// # Errors
    ///
    /// Returns an [`EventLoopError`] upon the loop exiting due to an error. See
    /// [`EventLoop::run`](appit::winit::event_loop::EventLoop::run) for more information.
    fn run() -> Result<(), EventLoopError>
    where
        Self::Context: Default,
    {
        Self::run_with(<Self::Context>::default())
    }

    /// Launches a Kludgine app using this window as the primary window.
    ///
    /// The `context` is passed along to [`initialize()`](Self::initialize) once
    /// the thread it is running on is spawned.
    ///
    /// # Panics
    ///
    /// On many platforms, it is a requirement that this function only be called
    /// from the thread that is executing the program's `main()` function.
    /// `wgpu` may panic when creating a surface if this function is not called
    /// from the correct thread.
    ///
    /// # Errors
    ///
    /// Returns an [`EventLoopError`] upon the loop exiting due to an error. See
    /// [`EventLoop::run`](appit::winit::event_loop::EventLoop::run) for more
    /// information.
    fn run_with(context: Self::Context) -> Result<(), EventLoopError> {
        let mut app = PendingApp::new();
        KludgineWindow::<Self>::new(&mut app, context).open()?;
        app.0.run()
    }

    /// Opens a new window with a default instance of this behavior's
    /// [`Context`](Self::Context). The events of the window will be processed
    /// in a thread spawned by this function.
    ///
    /// If the application has shut down, this function returns None.
    ///
    /// # Errors
    ///
    /// The only errors this funciton can return arise from winit's
    /// `create_window`.
    fn open<App>(app: &mut App) -> Result<Option<WindowHandle<WindowEvent>>, OsError>
    where
        App: AsApplication<AppEvent<WindowEvent>> + ?Sized,
        Self::Context: Default,
    {
        KludgineWindow::<Self>::new(app, <Self::Context>::default())
            .open()
            .map(|opt| opt.map(WindowHandle))
    }

    /// Opens a new window with the provided [`Context`](Self::Context). The
    /// events of the window will be processed in a thread spawned by this
    /// function.
    ///
    /// If the application has shut down, this function returns None.
    ///
    /// # Errors
    ///
    /// The only errors this funciton can return arise from winit's
    /// `create_window`.
    fn open_with<App>(
        app: &mut App,
        context: Self::Context,
    ) -> Result<Option<WindowHandle<WindowEvent>>, OsError>
    where
        App: AsApplication<AppEvent<WindowEvent>> + ?Sized,
    {
        KludgineWindow::<Self>::new(app, context)
            .open()
            .map(|opt| opt.map(WindowHandle))
    }

    /// The window has been requested to be closed. This can happen as a result
    /// of the user clicking the close button.
    ///
    /// If the window should be closed, return true. To prevent closing the
    /// window, return false.
    #[allow(unused_variables)]
    fn close_requested(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
    ) -> bool {
        true
    }

    /// The window has gained or lost keyboard focus. [`Window::focused()`]
    /// returns the current state.
    #[allow(unused_variables)]
    fn focus_changed(&mut self, window: Window<'_, WindowEvent>, kludgine: &mut Kludgine) {}

    /// The window has been occluded or revealed. [`Window::occluded()`] returns
    /// the current state.
    #[allow(unused_variables)]
    fn occlusion_changed(&mut self, window: Window<'_, WindowEvent>, kludgine: &mut Kludgine) {}

    /// The window's scale factor has changed. [`Window::scale()`] returns the
    /// current scale.
    #[allow(unused_variables)]
    fn scale_factor_changed(&mut self, window: Window<'_, WindowEvent>, kludgine: &mut Kludgine) {}

    /// The window has been resized. [`Window::inner_size()`] returns the
    /// current size.
    #[allow(unused_variables)]
    fn resized(&mut self, window: Window<'_, WindowEvent>, kludgine: &mut Kludgine) {}

    /// The window has been moved. [`Window::position()`] returns the current
    /// position.
    #[allow(unused_variables)]
    fn moved(&mut self, window: Window<'_, WindowEvent>, kludgine: &mut Kludgine) {}

    /// The window's theme has been updated. [`Window::theme()`] returns the
    /// current theme.
    #[allow(unused_variables)]
    fn theme_changed(&mut self, window: Window<'_, WindowEvent>, kludgine: &mut Kludgine) {}

    /// A file has been dropped on the window.
    #[allow(unused_variables)]
    fn dropped_file(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        path: PathBuf,
    ) {
    }

    /// A file is hovering over the window.
    #[allow(unused_variables)]
    fn hovered_file(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        path: PathBuf,
    ) {
    }

    /// A file being overed has been cancelled.
    #[allow(unused_variables)]
    fn hovered_file_cancelled(&mut self, window: Window<'_, WindowEvent>, kludgine: &mut Kludgine) {
    }

    /// An input event has generated a character.
    #[allow(unused_variables)]
    fn received_character(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        char: char,
    ) {
    }

    /// A keyboard event occurred while the window was focused.
    #[allow(unused_variables)]
    fn keyboard_input(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
    ) {
    }

    /// The keyboard modifier keys have changed. [`Window::modifiers()`] returns
    /// the current modifier keys state.
    #[allow(unused_variables)]
    fn modifiers_changed(&mut self, window: Window<'_, WindowEvent>, kludgine: &mut Kludgine) {}

    /// An international input even thas occurred for the window.
    #[allow(unused_variables)]
    fn ime(&mut self, window: Window<'_, WindowEvent>, kludgine: &mut Kludgine, ime: Ime) {}

    /// A cursor has moved over the window.
    #[allow(unused_variables)]
    fn cursor_moved(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        position: PhysicalPosition<f64>,
    ) {
    }

    /// A cursor has hovered over the window.
    #[allow(unused_variables)]
    fn cursor_entered(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
    ) {
    }

    /// A cursor is no longer hovering over the window.
    #[allow(unused_variables)]
    fn cursor_left(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
    ) {
    }

    /// An event from a mouse wheel.
    #[allow(unused_variables)]
    fn mouse_wheel(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
    ) {
    }

    /// A mouse button was pressed or released.
    #[allow(unused_variables)]
    fn mouse_input(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        state: ElementState,
        button: MouseButton,
    ) {
    }

    /// A pressure-sensitive touchpad was touched.
    #[allow(unused_variables)]
    fn touchpad_pressure(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        pressure: f32,
        stage: i64,
    ) {
    }

    /// A multi-axis input device has registered motion.
    #[allow(unused_variables)]
    fn axis_motion(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        axis: AxisId,
        value: f64,
    ) {
    }

    /// A touch event.
    #[allow(unused_variables)]
    fn touch(&mut self, window: Window<'_, WindowEvent>, kludgine: &mut Kludgine, touch: Touch) {}

    /// A pinch-to-zoom gesture.
    #[allow(unused_variables)]
    fn pinch_gesture(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        delta: f64,
        phase: TouchPhase,
    ) {
    }

    /// A pan/scroll gesture.
    #[allow(unused_variables)]
    fn pan_gesture(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        delta: Point<f32>,
        phase: TouchPhase,
    ) {
    }

    /// A double-tap gesture directed at the window.
    #[allow(unused_variables)]
    fn double_tap_gesture(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
    ) {
    }

    /// A touchpad-originated rotation gesture.
    #[allow(unused_variables)]
    fn touchpad_rotate(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        delta: f32,
        phase: TouchPhase,
    ) {
    }

    /// A `WindowEvent` has been received by this window.
    #[allow(unused_variables)]
    fn event(
        &mut self,
        window: Window<'_, WindowEvent>,
        kludgine: &mut Kludgine,
        event: WindowEvent,
    ) {
    }
}

/// A Kludgine application event.
pub struct AppEvent<User>(AppEventKind<User>);

enum AppEventKind<User> {
    CreateSurface(CreateSurfaceRequest<User>),
    ListMonitors,
}

/// A response to an [`AppEvent`].
pub struct AppResponse(AppResponseKind);

impl AppResponse {
    fn expect_surface(self) -> wgpu::Surface<'static> {
        let AppResponse(AppResponseKind::Surface(surface)) = self else {
            unreachable!("unexpected response")
        };
        surface
    }

    fn expect_monitors(self) -> Monitors {
        let AppResponse(AppResponseKind::Monitors(monitors)) = self else {
            unreachable!("unexpected response")
        };
        monitors
    }
}

/// A snapshot of information about monitors (displays) connected to this
/// device.
#[derive(Clone, Debug)]
pub struct Monitors {
    /// The primary monitor.
    pub primary: Option<Monitor>,
    /// All available monitors.
    pub available: Vec<Monitor>,
}

/// Information about a monitor connected to a device.
#[derive(Clone, Debug, PartialEq)]
pub struct Monitor(MonitorHandle);

impl Monitor {
    /// Returns the name of the monitor, if available.
    #[must_use]
    pub fn name(&self) -> Option<String> {
        self.0.name()
    }

    /// Returns the position of the top-left corner of the monitor.
    #[must_use]
    pub fn position(&self) -> Point<Px> {
        self.0.position().into()
    }

    /// Returns the size of this monitor.
    #[must_use]
    pub fn size(&self) -> Size<UPx> {
        self.0.size().into()
    }

    /// Returns a rectangle representing the position and size of this monitor.q
    #[must_use]
    pub fn region(&self) -> Rect<Px> {
        Rect::new(self.position(), self.size().into_signed())
    }

    /// Returns the DPI scaling factor applied to this monitor.
    #[allow(clippy::cast_possible_truncation)]
    #[must_use]
    pub fn scale_factor(&self) -> Fraction {
        Fraction::from(self.0.scale_factor() as f32)
    }

    /// Returns the refresh rate of this display, in millihertz.
    #[must_use]
    pub fn refresh_rate_millihertz(&self) -> Option<u32> {
        self.0.refresh_rate_millihertz()
    }

    /// Returns an iterator of the video modes supported by this monitor.
    pub fn video_modes(&self) -> impl Iterator<Item = VideoMode> {
        self.0.video_modes().map(VideoMode)
    }

    /// Returns a reference to the underlying handle.
    #[must_use]
    pub const fn handle(&self) -> &MonitorHandle {
        &self.0
    }
}

/// A specific video mode for a [`Monitor`].
#[derive(Clone, Debug)]
pub struct VideoMode(VideoModeHandle);

impl VideoMode {
    /// Returns a reference to the underlying handle.
    #[must_use]
    pub const fn handle(&self) -> &VideoModeHandle {
        &self.0
    }

    /// Returns the color bit depth of this video mode.
    ///
    /// For most platforms this is generally 24 or 32 bits.
    #[must_use]
    pub fn bit_depth(&self) -> u16 {
        self.0.bit_depth()
    }

    /// Returns the size the monitor will display at with this video mode.
    #[must_use]
    pub fn size(&self) -> Size<UPx> {
        self.0.size().into()
    }

    /// Returns the refresh rate of this video mode.
    #[must_use]
    pub fn refresh_rate_millihertz(&self) -> u32 {
        self.0.refresh_rate_millihertz()
    }

    /// Returns the monitor associated with this video mode.
    #[must_use]
    pub fn monitor(&self) -> Monitor {
        Monitor(self.0.monitor())
    }
}

enum AppResponseKind {
    Surface(wgpu::Surface<'static>),
    Monitors(Monitors),
}

struct CreateSurfaceRequest<User> {
    wgpu: Arc<wgpu::Instance>,
    window: WindowId,
    data: PhantomData<User>,
}

impl<User> Message for AppEvent<User>
where
    User: Send + 'static,
{
    type Response = AppResponse;
    type Window = User;
}

struct KludgineWindow<Behavior> {
    behavior: Behavior,
    kludgine: Kludgine,
    last_render: Instant,
    last_render_duration: Duration,

    config: wgpu::SurfaceConfiguration,
    surface: wgpu::Surface<'static>,
    msaa_texture: Option<wgpu::Texture>,
    queue: wgpu::Queue,
    wgpu: Arc<wgpu::Instance>,
    device: wgpu::Device,
    multisample_count: u32,
}

impl<Behavior> KludgineWindow<Behavior> {
    fn new<App, User>(
        app: &mut App,
        context: Behavior::Context,
    ) -> appit::WindowBuilder<'_, Self, App, AppEvent<User>>
    where
        App: AsApplication<AppEvent<User>> + ?Sized,
        Behavior: WindowBehavior<User> + 'static,
        User: Send + 'static,
    {
        let window_attributes = Behavior::initial_window_attributes(&context);

        let mut window = Self::build_with(app, context);
        *window = window_attributes;
        window
    }

    fn current_surface_texture<User>(
        &mut self,
        window: &mut RunningWindow<AppEvent<User>>,
    ) -> Option<wgpu::SurfaceTexture>
    where
        AppEvent<User>: Message<Response = AppResponse>,
    {
        loop {
            match self.surface.get_current_texture() {
                Ok(frame) => break Some(frame),
                Err(other) => match other {
                    wgpu::SurfaceError::Timeout => continue,
                    wgpu::SurfaceError::Outdated => {
                        // Needs to be reconfigured. We do this automatically
                        // when the window is resized. We need to allow the
                        // event loop to catch up.
                        return None;
                    }
                    wgpu::SurfaceError::Lost => {
                        self.surface = window
                            .send(AppEvent(AppEventKind::CreateSurface(
                                CreateSurfaceRequest {
                                    wgpu: self.wgpu.clone(),
                                    window: window.winit().id(),
                                    data: PhantomData,
                                },
                            )))
                            .expect("app not running")
                            .expect_surface();
                        self.surface.configure(&self.device, &self.config);
                    }
                    wgpu::SurfaceError::OutOfMemory => {
                        unreachable!(
                            "out of memory error when requesting current swap chain texture"
                        )
                    }
                },
            }
        }
    }

    fn render_to_surface<User>(
        &mut self,
        surface: wgpu::SurfaceTexture,
        render_start: Instant,
        window: &mut RunningWindow<AppEvent<User>>,
    ) where
        AppEvent<User>: Message,
        Behavior: WindowBehavior<User> + 'static,
        User: Send + 'static,
    {
        let mut frame = self.kludgine.next_frame();
        let elapsed = render_start - self.last_render;

        self.behavior.prepare(
            Window::new(window, elapsed, self.last_render_duration),
            &mut frame.prepare(&self.device, &self.queue),
        );

        let surface_view = surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let (view, resolve_target) = if self.multisample_count > 1 {
            if self.msaa_texture.as_ref().map_or(true, |msaa| {
                msaa.width() != surface.texture.width() || msaa.height() != surface.texture.height()
            }) {
                self.msaa_texture = Some(self.device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: surface.texture.width(),
                        height: surface.texture.height(),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: self.multisample_count,
                    dimension: wgpu::TextureDimension::D2,
                    format: surface.texture.format(),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                }));
            }

            (
                self.msaa_texture
                    .as_ref()
                    .assert("always initialized")
                    .create_view(&wgpu::TextureViewDescriptor::default()),
                Some(surface_view),
            )
        } else {
            (surface_view, None)
        };

        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: resolve_target.as_ref(),
            ops: wgpu::Operations {
                load: self
                    .behavior
                    .clear_color()
                    .map_or(wgpu::LoadOp::Load, |color| {
                        wgpu::LoadOp::Clear(color.into())
                    }),
                store: wgpu::StoreOp::Store,
            },
        })];
        let mut gfx = frame.render(
            &wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &color_attachments,
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            },
            &self.device,
            &self.queue,
        );
        let close_after_frame = !self.behavior.render(
            Window::new(window, elapsed, self.last_render_duration),
            &mut gfx,
        );
        drop(gfx);
        let id = frame.submit(&self.queue);
        window.winit().pre_present_notify();
        surface.present();
        if let Some(id) = id {
            self.device.poll(wgpu::Maintain::WaitForSubmissionIndex(id));
        }
        if close_after_frame {
            window.close();
        }
    }
}

fn new_wgpu_instance() -> wgpu::Instance {
    let flags;
    #[cfg(debug_assertions)]
    {
        flags = wgpu::InstanceFlags::debugging();
    }
    #[cfg(not(debug_assertions))]
    {
        flags = wgpu::InstanceFlags::empty();
    }
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        flags,
        ..wgpu::InstanceDescriptor::default()
    })
}

impl<T> KludgineWindow<T> {
    fn resized<User>(&mut self, window: &mut RunningWindow<AppEvent<User>>)
    where
        T: WindowBehavior<User> + 'static,
        User: Send + 'static,
    {
        self.config.width = window.inner_size().width;
        self.config.height = window.inner_size().height;
        if self.config.width > 0 && self.config.height > 0 {
            self.surface.configure(&self.device, &self.config);
            self.kludgine.resize(
                window.inner_size().into(),
                window.scale().cast::<f32>(),
                self.kludgine.zoom,
                &self.queue,
            );
            window.set_needs_redraw();
        }
        self.behavior.resized(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
        );
    }
}

impl<T, User> appit::WindowBehavior<AppEvent<User>> for KludgineWindow<T>
where
    T: WindowBehavior<User> + 'static,
    User: Send + 'static,
{
    type Context = T::Context;

    fn initialize(window: &mut RunningWindow<AppEvent<User>>, context: Self::Context) -> Self {
        let wgpu = Arc::new(new_wgpu_instance());
        let surface = window
            .send(AppEvent(AppEventKind::CreateSurface(
                CreateSurfaceRequest {
                    wgpu: wgpu.clone(),
                    window: window.winit().id(),
                    data: PhantomData,
                },
            )))
            .expect("app not running")
            .expect_surface();
        let adapter = pollster::block_on(wgpu.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: T::power_preference(&context),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .expect("no compatible graphics adapters found");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: Kludgine::REQURED_FEATURES,
                required_limits: Kludgine::adjust_limits(T::limits(adapter.limits(), &context)),
                memory_hints: T::memory_hints(&context),
            },
            None,
        ))
        .expect("no compatible graphics devices found");

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
        let multisample_count = T::multisample_count(&context).get();
        let multisample = wgpu::MultisampleState {
            count: multisample_count,
            ..Default::default()
        };

        let mut state = Kludgine::new(
            &device,
            &queue,
            swapchain_format,
            multisample,
            window.inner_size().into(),
            window.scale().cast::<f32>(),
        );
        let mut graphics = Graphics::new(&mut state, &device, &queue);

        let last_render = Instant::now();
        let behavior = T::initialize(
            Window {
                window,
                elapsed: Duration::ZERO,
                last_frame_rendered_in: Duration::ZERO,
            },
            &mut graphics,
            context,
        );

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: behavior.present_mode(),
            alpha_mode: behavior.composite_alpha_mode(&swapchain_capabilities.alpha_modes),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Self {
            kludgine: state,
            last_render,
            last_render_duration: Duration::ZERO,
            msaa_texture: None,
            behavior,
            config,
            surface,
            device,
            queue,
            wgpu,
            multisample_count,
        }
    }

    fn initialized(&mut self, window: &mut RunningWindow<AppEvent<User>>) {
        self.behavior.initialized(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
        );
    }

    fn redraw(&mut self, window: &mut RunningWindow<AppEvent<User>>) {
        if self.config.width > 0 && self.config.height > 0 {
            let current_size = Size::<UPx>::from(window.inner_size());
            if current_size != self.kludgine.size() {
                self.resized(window);
            }
            let Some(surface) = self.current_surface_texture(window) else {
                return;
            };

            let render_start = Instant::now();

            self.render_to_surface(surface, render_start, window);

            self.last_render_duration = render_start.elapsed();
            self.last_render = render_start;
        }
    }

    fn close_requested(&mut self, window: &mut RunningWindow<AppEvent<User>>) -> bool {
        self.behavior.close_requested(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
        )
    }

    fn focus_changed(&mut self, window: &mut RunningWindow<AppEvent<User>>) {
        self.behavior.focus_changed(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
        );
    }

    fn occlusion_changed(&mut self, window: &mut RunningWindow<AppEvent<User>>) {
        self.behavior.occlusion_changed(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
        );
    }

    fn resized(&mut self, window: &mut RunningWindow<AppEvent<User>>) {
        self.resized(window);
    }

    fn moved(&mut self, window: &mut RunningWindow<AppEvent<User>>) {
        self.behavior.moved(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
        );
    }

    fn scale_factor_changed(&mut self, window: &mut RunningWindow<AppEvent<User>>) {
        self.behavior.scale_factor_changed(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
        );
    }

    fn theme_changed(&mut self, window: &mut RunningWindow<AppEvent<User>>) {
        self.behavior.theme_changed(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
        );
    }

    fn dropped_file(&mut self, window: &mut RunningWindow<AppEvent<User>>, path: PathBuf) {
        self.behavior.dropped_file(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            path,
        );
    }

    fn hovered_file(&mut self, window: &mut RunningWindow<AppEvent<User>>, path: PathBuf) {
        self.behavior.hovered_file(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            path,
        );
    }

    fn hovered_file_cancelled(&mut self, window: &mut RunningWindow<AppEvent<User>>) {
        self.behavior.hovered_file_cancelled(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
        );
    }

    fn received_character(&mut self, window: &mut RunningWindow<AppEvent<User>>, char: char) {
        self.behavior.received_character(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            char,
        );
    }

    fn keyboard_input(
        &mut self,
        window: &mut RunningWindow<AppEvent<User>>,
        device_id: DeviceId,
        event: KeyEvent,
        is_synthetic: bool,
    ) {
        self.behavior.keyboard_input(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            device_id,
            event,
            is_synthetic,
        );
    }

    fn modifiers_changed(&mut self, window: &mut RunningWindow<AppEvent<User>>) {
        self.behavior.modifiers_changed(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
        );
    }

    fn ime(&mut self, window: &mut RunningWindow<AppEvent<User>>, ime: Ime) {
        self.behavior.ime(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            ime,
        );
    }

    fn cursor_moved(
        &mut self,
        window: &mut RunningWindow<AppEvent<User>>,
        device_id: DeviceId,
        position: PhysicalPosition<f64>,
    ) {
        self.behavior.cursor_moved(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            device_id,
            position,
        );
    }

    fn cursor_entered(&mut self, window: &mut RunningWindow<AppEvent<User>>, device_id: DeviceId) {
        self.behavior.cursor_entered(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            device_id,
        );
    }

    fn cursor_left(&mut self, window: &mut RunningWindow<AppEvent<User>>, device_id: DeviceId) {
        self.behavior.cursor_left(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            device_id,
        );
    }

    fn mouse_wheel(
        &mut self,
        window: &mut RunningWindow<AppEvent<User>>,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
    ) {
        self.behavior.mouse_wheel(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            device_id,
            delta,
            phase,
        );
    }

    fn mouse_input(
        &mut self,
        window: &mut RunningWindow<AppEvent<User>>,
        device_id: DeviceId,
        state: ElementState,
        button: MouseButton,
    ) {
        self.behavior.mouse_input(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            device_id,
            state,
            button,
        );
    }

    fn touchpad_pressure(
        &mut self,
        window: &mut RunningWindow<AppEvent<User>>,
        device_id: DeviceId,
        pressure: f32,
        stage: i64,
    ) {
        self.behavior.touchpad_pressure(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            device_id,
            pressure,
            stage,
        );
    }

    fn axis_motion(
        &mut self,
        window: &mut RunningWindow<AppEvent<User>>,
        device_id: DeviceId,
        axis: AxisId,
        value: f64,
    ) {
        self.behavior.axis_motion(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            device_id,
            axis,
            value,
        );
    }

    fn touch(&mut self, window: &mut RunningWindow<AppEvent<User>>, touch: Touch) {
        self.behavior.touch(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            touch,
        );
    }

    fn pinch_gesture(
        &mut self,
        window: &mut RunningWindow<AppEvent<User>>,
        device_id: DeviceId,
        delta: f64,
        phase: TouchPhase,
    ) {
        self.behavior.pinch_gesture(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            device_id,
            delta,
            phase,
        );
    }

    fn pan_gesture(
        &mut self,
        window: &mut RunningWindow<AppEvent<User>>,
        device_id: DeviceId,
        delta: PhysicalPosition<f32>,
        phase: TouchPhase,
    ) {
        self.behavior.pan_gesture(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            device_id,
            Point::new(delta.x, delta.y),
            phase,
        );
    }

    fn double_tap_gesture(
        &mut self,
        window: &mut RunningWindow<AppEvent<User>>,
        device_id: DeviceId,
    ) {
        self.behavior.double_tap_gesture(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            device_id,
        );
    }

    fn touchpad_rotate(
        &mut self,
        window: &mut RunningWindow<AppEvent<User>>,
        device_id: DeviceId,
        delta: f32,
        phase: TouchPhase,
    ) {
        self.behavior.touchpad_rotate(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            device_id,
            delta,
            phase,
        );
    }

    fn event(
        &mut self,
        window: &mut RunningWindow<AppEvent<User>>,
        event: <AppEvent<User> as Message>::Window,
    ) {
        self.behavior.event(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            &mut self.kludgine,
            event,
        );
    }
}

struct CallbackWindow<C> {
    callback: C,
    rendering: Drawing,
    keep_running: bool,
}

impl<T> WindowBehavior for CallbackWindow<T>
where
    T: for<'render, 'gfx, 'window> FnMut(Renderer<'render, 'gfx>, Window<'window>) -> bool
        + Send
        + 'static,
{
    type Context = T;

    fn initialize(
        _window: Window<'_>,
        _graphics: &mut Graphics<'_>,
        context: Self::Context,
    ) -> Self {
        Self {
            callback: context,
            rendering: Drawing::default(),
            keep_running: true,
        }
    }

    fn prepare(&mut self, window: Window<'_>, graphics: &mut Graphics<'_>) {
        let renderer = self.rendering.new_frame(graphics);
        self.keep_running = (self.callback)(renderer, window);
    }

    fn render<'pass>(
        &'pass mut self,
        _window: Window<'_>,
        graphics: &mut RenderingGraphics<'_, 'pass>,
    ) -> bool {
        self.rendering.render(1., graphics);
        self.keep_running
    }
}

/// Runs a callback as a single window. Continues to run until false is
/// returned.
///
/// # Errors
///
/// Returns an [`EventLoopError`] upon the loop exiting due to an error. See
/// [`EventLoop::run`](appit::winit::event_loop::EventLoop::run) for more
/// information.
pub fn run<RenderFn>(render_fn: RenderFn) -> Result<(), EventLoopError>
where
    RenderFn: for<'render, 'gfx, 'window> FnMut(Renderer<'render, 'gfx>, Window<'window>) -> bool
        + Send
        + 'static,
{
    CallbackWindow::run_with(render_fn)
}

/// A handle to a window.
///
/// This handle does not prevent the window from being closed.
#[derive(Debug)]
pub struct WindowHandle<Message = ()>(appit::Window<Message>);

impl<Message> WindowHandle<Message> {
    /// Sends `message` to the window. If the message cannot be
    ///
    /// Returns `Ok` if the message was successfully sent. The message may not
    /// be received even if this function returns `Ok`, if the window closes
    /// between when the message was sent and when the message is received.
    ///
    /// # Errors
    ///
    /// If the window is already closed, this function returns `Err(message)`.
    pub fn send(&self, message: Message) -> Result<(), Message> {
        self.0.send(message)
    }
}

impl<Message> Clone for WindowHandle<Message> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
