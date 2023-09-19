use std::marker::PhantomData;
use std::mem::size_of;
use std::panic::UnwindSafe;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use appit::winit::dpi::PhysicalPosition;
use appit::winit::error::EventLoopError;
use appit::winit::event::{
    AxisId, DeviceId, ElementState, Ime, KeyEvent, Modifiers, MouseButton, MouseScrollDelta, Touch,
    TouchPhase,
};
use appit::winit::keyboard::KeyCode;
use appit::winit::window::WindowId;
pub use appit::{winit, Message, WindowAttributes};
use appit::{Application, PendingApp, RunningWindow, WindowBehavior as _};
use figures::units::{Px, UPx};
use figures::utils::lossy_f64_to_f32;
use figures::{Point, Size};

use crate::pipeline::PushConstants;
use crate::render::{Drawing, Renderer};
use crate::{Color, Graphics, Kludgine, RenderingGraphics};

fn shared_wgpu() -> Arc<wgpu::Instance> {
    static SHARED_WGPU: OnceLock<Arc<wgpu::Instance>> = OnceLock::new();
    SHARED_WGPU.get_or_init(Arc::default).clone()
}

/// An open window.
pub struct Window<'window, WindowEvent = ()>
where
    WindowEvent: Send + 'static,
{
    window: &'window mut RunningWindow<CreateSurfaceRequest<WindowEvent>>,
    elapsed: Duration,
    last_frame_rendered_in: Duration,
}

impl<'window, WindowEvent> Window<'window, WindowEvent>
where
    WindowEvent: Send + 'static,
{
    fn new(
        window: &'window mut RunningWindow<CreateSurfaceRequest<WindowEvent>>,
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

    /// Returns the current position of the window.
    #[must_use]
    pub fn position(&self) -> Point<Px> {
        self.window.position().into()
    }

    /// Sets the current position of the window.
    pub fn set_position(&self, position: Point<Px>) {
        self.window.set_position(position.into());
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

    /// Returns the current title of the window.
    #[must_use]
    pub fn title(&self) -> String {
        self.window.title()
    }

    /// Sets the title of the window.
    pub fn set_title(&mut self, new_title: &str) {
        self.window.set_title(new_title);
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
    pub fn key_pressed(&self, key: &KeyCode) -> bool {
        self.window.key_pressed(key)
    }

    /// Returns currently active modifiers.
    #[must_use]
    pub fn modifiers(&self) -> Modifiers {
        self.window.modifiers()
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
    type Context: UnwindSafe + Send + 'static;

    /// Initialize a new instance from the provided context.
    fn initialize(
        window: Window<'_, WindowEvent>,
        graphics: &mut Graphics<'_>,
        context: Self::Context,
    ) -> Self;

    /// Returns the window attributes to use when creating the window.
    #[must_use]
    #[allow(unused_variables)]
    fn initial_window_attributes(context: &Self::Context) -> WindowAttributes<WindowEvent> {
        WindowAttributes::default()
    }

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

    /// Returns the power preference to initialize `wgpu` with.
    #[must_use]
    fn power_preference() -> wgpu::PowerPreference {
        wgpu::PowerPreference::default()
    }

    /// Returns the limits to apply for the `wgpu` instance.
    #[must_use]
    fn limits(adapter_limits: wgpu::Limits) -> wgpu::Limits {
        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter_limits)
    }

    /// Returns the color to clear the window with. If None is returned, the
    /// window will not be cleared between redraws.
    ///
    /// The default implementation returns `Some(Color::BLACK)`.
    #[must_use]
    fn clear_color() -> Option<Color> {
        Some(Color::BLACK)
    }

    /// Launches a Kludgine app using this window as the primary window.
    ///
    /// # Errors
    ///
    /// Returns an [`EventLoopError`] upon the loop exiting due to an error. See
    /// [`EventLoop::run`] for more information.
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
    /// # Errors
    ///
    /// Returns an [`EventLoopError`] upon the loop exiting due to an error. See
    /// [`EventLoop::run`] for more information.
    fn run_with(context: Self::Context) -> Result<(), EventLoopError> {
        let window_attributes = Self::initial_window_attributes(&context);

        let app = PendingApp::new_with_event_callback(handle_window_message);
        let mut window = KludgineWindow::<Self>::build_with(&app, context);
        *window = window_attributes;
        window.open().expect("error opening initial window");
        app.run()
    }

    /// The window has been requested to be closed. This can happen as a result
    /// of the user clicking the close button.
    ///
    /// If the window should be closed, return true. To prevent closing the
    /// window, return false.
    #[allow(unused_variables)]
    fn close_requested(&mut self, window: Window<'_, WindowEvent>) -> bool {
        true
    }

    /// The window has gained or lost keyboard focus.
    /// [`RunningWindow::focused()`] returns the current state.
    #[allow(unused_variables)]
    fn focus_changed(&mut self, window: Window<'_, WindowEvent>) {}

    /// The window has been occluded or revealed. [`RunningWindow::occluded()`]
    /// returns the current state.
    #[allow(unused_variables)]
    fn occlusion_changed(&mut self, window: Window<'_, WindowEvent>) {}

    /// The window's scale factor has changed. [`RunningWindow::scale()`]
    /// returns the current scale.
    #[allow(unused_variables)]
    fn scale_factor_changed(&mut self, window: Window<'_, WindowEvent>) {}

    /// The window has been resized. [`RunningWindow::inner_size()`]
    /// returns the current size.
    #[allow(unused_variables)]
    fn resized(&mut self, window: Window<'_, WindowEvent>) {}

    /// The window's theme has been updated. [`RunningWindow::theme()`]
    /// returns the current theme.
    #[allow(unused_variables)]
    fn theme_changed(&mut self, window: Window<'_, WindowEvent>) {}

    /// A file has been dropped on the window.
    #[allow(unused_variables)]
    fn dropped_file(&mut self, window: Window<'_, WindowEvent>, path: PathBuf) {}

    /// A file is hovering over the window.
    #[allow(unused_variables)]
    fn hovered_file(&mut self, window: Window<'_, WindowEvent>, path: PathBuf) {}

    /// A file being overed has been cancelled.
    #[allow(unused_variables)]
    fn hovered_file_cancelled(&mut self, window: Window<'_, WindowEvent>) {}

    /// An input event has generated a character.
    #[allow(unused_variables)]
    fn received_character(&mut self, window: Window<'_, WindowEvent>, char: char) {}

    /// A keyboard event occurred while the window was focused.
    #[allow(unused_variables)]
    fn keyboard_input(
        &mut self,
        window: Window<'_, WindowEvent>,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
    ) {
    }

    /// The keyboard modifier keys have changed. [`RunningWindow::modifiers()`]
    /// returns the current modifier keys state.
    #[allow(unused_variables)]
    fn modifiers_changed(&mut self, window: Window<'_, WindowEvent>) {}

    /// An international input even thas occurred for the window.
    #[allow(unused_variables)]
    fn ime(&mut self, window: Window<'_, WindowEvent>, ime: Ime) {}

    /// A cursor has moved over the window.
    #[allow(unused_variables)]
    fn cursor_moved(
        &mut self,
        window: Window<'_, WindowEvent>,
        device_id: DeviceId,
        position: PhysicalPosition<f64>,
    ) {
    }

    /// A cursor has hovered over the window.
    #[allow(unused_variables)]
    fn cursor_entered(&mut self, window: Window<'_, WindowEvent>, device_id: DeviceId) {}

    /// A cursor is no longer hovering over the window.
    #[allow(unused_variables)]
    fn cursor_left(&mut self, window: Window<'_, WindowEvent>, device_id: DeviceId) {}

    /// An event from a mouse wheel.
    #[allow(unused_variables)]
    fn mouse_wheel(
        &mut self,
        window: Window<'_, WindowEvent>,
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
        device_id: DeviceId,
        axis: AxisId,
        value: f64,
    ) {
    }

    /// A touch event.
    #[allow(unused_variables)]
    fn touch(&mut self, window: Window<'_, WindowEvent>, touch: Touch) {}

    /// A touchpad-originated magnification gesture.
    #[allow(unused_variables)]
    fn touchpad_magnify(
        &mut self,
        window: Window<'_, WindowEvent>,
        device_id: DeviceId,
        delta: f64,
        phase: TouchPhase,
    ) {
    }

    /// A request to smart-magnify the window.
    #[allow(unused_variables)]
    fn smart_magnify(&mut self, window: Window<'_, WindowEvent>, device_id: DeviceId) {}

    /// A touchpad-originated rotation gesture.
    #[allow(unused_variables)]
    fn touchpad_rotate(
        &mut self,
        window: Window<'_, WindowEvent>,
        device_id: DeviceId,
        delta: f32,
        phase: TouchPhase,
    ) {
    }

    /// A `WindowEvent` has been received by this window.
    #[allow(unused_variables)]
    fn event(&mut self, event: WindowEvent, window: Window<'_, WindowEvent>) {}
}

#[allow(unsafe_code, clippy::needless_pass_by_value)]
fn handle_window_message<User>(
    request: CreateSurfaceRequest<User>,
    windows: &appit::Windows<User>,
) -> wgpu::Surface
where
    User: Send + 'static,
{
    let window = windows.get(request.window).expect("window not found");
    unsafe {
        shared_wgpu()
            .create_surface(&*window)
            .expect("error creating surface")
    }
}

struct CreateSurfaceRequest<User> {
    window: WindowId,
    data: PhantomData<User>,
}

impl<User> Message for CreateSurfaceRequest<User>
where
    User: Send + 'static,
{
    type Response = wgpu::Surface;
    type Window = User;
}

struct KludgineWindow<Behavior> {
    behavior: Behavior,
    kludgine: Kludgine,
    last_render: Instant,
    last_render_duration: Duration,

    config: wgpu::SurfaceConfiguration,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    _adapter: wgpu::Adapter,
    wgpu: Arc<wgpu::Instance>,
}

impl<T, User> appit::WindowBehavior<CreateSurfaceRequest<User>> for KludgineWindow<T>
where
    T: WindowBehavior<User> + 'static,
    User: Send + 'static,
{
    type Context = T::Context;

    #[allow(unsafe_code)]
    fn initialize(
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
        context: Self::Context,
    ) -> Self {
        // SAFETY: This function is only invoked once the window has been
        // created, and cannot be invoked after the underlying window has been
        // destroyed.
        let surface = window
            .send(CreateSurfaceRequest {
                window: window.winit().id(),
                data: PhantomData,
            })
            .expect("app not running");
        let wgpu = shared_wgpu();
        let adapter = pollster::block_on(wgpu.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: T::power_preference(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .unwrap();
        let mut limits = T::limits(adapter.limits());
        limits.max_push_constant_size = size_of::<PushConstants>()
            .try_into()
            .expect("should fit :)");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::PUSH_CONSTANTS,
                limits,
            },
            None,
        ))
        .unwrap();

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let mut state = Kludgine::new(
            &device,
            &queue,
            swapchain_format,
            Size::new(window.inner_size().width, window.inner_size().height),
            lossy_f64_to_f32(window.scale()),
        );
        let mut graphics = Graphics::new(&mut state, &device, &queue);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

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

        Self {
            wgpu,
            kludgine: state,
            last_render,
            last_render_duration: Duration::ZERO,
            _adapter: adapter,
            behavior,
            config,
            surface,
            device,
            queue,
        }
    }

    #[allow(unsafe_code)]
    fn redraw(&mut self, window: &mut RunningWindow<CreateSurfaceRequest<User>>) {
        let surface = loop {
            match self.surface.get_current_texture() {
                Ok(frame) => break frame,
                Err(other) => match other {
                    wgpu::SurfaceError::Timeout => continue,
                    wgpu::SurfaceError::Outdated => {
                        // Needs to be reconfigured. We do this automatically
                        // when the window is resized. We need to allow the
                        // event loop to catch up.
                        return;
                    }
                    wgpu::SurfaceError::Lost => {
                        // SAFETY: redraw is only called while the event loop
                        // and window are still alive.
                        self.surface = unsafe { self.wgpu.create_surface(window.winit()).unwrap() };
                        self.surface.configure(&self.device, &self.config);
                    }
                    wgpu::SurfaceError::OutOfMemory => {
                        unreachable!(
                            "out of memory error when requesting current swap chain texture"
                        )
                    }
                },
            }
        };

        let mut frame = self.kludgine.next_frame();
        let render_start = Instant::now();
        let elapsed = render_start - self.last_render;

        self.behavior.prepare(
            Window::new(window, elapsed, self.last_render_duration),
            &mut frame.prepare(&self.device, &self.queue),
        );

        let view = surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut gfx = frame.render(
            &wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: T::clear_color().map_or(wgpu::LoadOp::Load, |color| {
                            wgpu::LoadOp::Clear(color.into())
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            },
            &self.device,
            &self.queue,
        );
        self.behavior.render(
            Window::new(window, elapsed, self.last_render_duration),
            &mut gfx,
        );
        drop(gfx);
        frame.submit(&self.queue);
        surface.present();
        self.last_render_duration = render_start.elapsed();
        self.last_render = render_start;
    }

    fn close_requested(&mut self, window: &mut RunningWindow<CreateSurfaceRequest<User>>) -> bool {
        self.behavior.close_requested(Window::new(
            window,
            self.last_render.elapsed(),
            self.last_render_duration,
        ))
    }

    fn focus_changed(&mut self, window: &mut RunningWindow<CreateSurfaceRequest<User>>) {
        self.behavior.focus_changed(Window::new(
            window,
            self.last_render.elapsed(),
            self.last_render_duration,
        ));
    }

    fn occlusion_changed(&mut self, window: &mut RunningWindow<CreateSurfaceRequest<User>>) {
        self.behavior.occlusion_changed(Window::new(
            window,
            self.last_render.elapsed(),
            self.last_render_duration,
        ));
    }

    fn resized(&mut self, window: &mut RunningWindow<CreateSurfaceRequest<User>>) {
        self.config.width = window.inner_size().width;
        self.config.height = window.inner_size().height;
        self.surface.configure(&self.device, &self.config);
        self.kludgine.resize(
            Size::new(window.inner_size().width, window.inner_size().height),
            lossy_f64_to_f32(window.scale()),
            &self.queue,
        );
        window.set_needs_redraw();
        self.behavior.resized(Window::new(
            window,
            self.last_render.elapsed(),
            self.last_render_duration,
        ));
    }

    fn scale_factor_changed(&mut self, window: &mut RunningWindow<CreateSurfaceRequest<User>>) {
        self.behavior.scale_factor_changed(Window::new(
            window,
            self.last_render.elapsed(),
            self.last_render_duration,
        ));
    }

    fn theme_changed(&mut self, window: &mut RunningWindow<CreateSurfaceRequest<User>>) {
        self.behavior.theme_changed(Window::new(
            window,
            self.last_render.elapsed(),
            self.last_render_duration,
        ));
    }

    fn dropped_file(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
        path: PathBuf,
    ) {
        self.behavior.dropped_file(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            path,
        );
    }

    fn hovered_file(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
        path: PathBuf,
    ) {
        self.behavior.hovered_file(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            path,
        );
    }

    fn hovered_file_cancelled(&mut self, window: &mut RunningWindow<CreateSurfaceRequest<User>>) {
        self.behavior.hovered_file_cancelled(Window::new(
            window,
            self.last_render.elapsed(),
            self.last_render_duration,
        ));
    }

    fn received_character(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
        char: char,
    ) {
        self.behavior.received_character(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            char,
        );
    }

    fn keyboard_input(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
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
            device_id,
            event,
            is_synthetic,
        );
    }

    fn modifiers_changed(&mut self, window: &mut RunningWindow<CreateSurfaceRequest<User>>) {
        self.behavior.modifiers_changed(Window::new(
            window,
            self.last_render.elapsed(),
            self.last_render_duration,
        ));
    }

    fn ime(&mut self, window: &mut RunningWindow<CreateSurfaceRequest<User>>, ime: Ime) {
        self.behavior.ime(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            ime,
        );
    }

    fn cursor_moved(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
        device_id: DeviceId,
        position: PhysicalPosition<f64>,
    ) {
        self.behavior.cursor_moved(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            device_id,
            position,
        );
    }

    fn cursor_entered(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
        device_id: DeviceId,
    ) {
        self.behavior.cursor_entered(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            device_id,
        );
    }

    fn cursor_left(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
        device_id: DeviceId,
    ) {
        self.behavior.cursor_left(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            device_id,
        );
    }

    fn mouse_wheel(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
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
            device_id,
            delta,
            phase,
        );
    }

    fn mouse_input(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
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
            device_id,
            state,
            button,
        );
    }

    fn touchpad_pressure(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
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
            device_id,
            pressure,
            stage,
        );
    }

    fn axis_motion(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
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
            device_id,
            axis,
            value,
        );
    }

    fn touch(&mut self, window: &mut RunningWindow<CreateSurfaceRequest<User>>, touch: Touch) {
        self.behavior.touch(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            touch,
        );
    }

    fn touchpad_magnify(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
        device_id: DeviceId,
        delta: f64,
        phase: TouchPhase,
    ) {
        self.behavior.touchpad_magnify(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            device_id,
            delta,
            phase,
        );
    }

    fn smart_magnify(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
        device_id: DeviceId,
    ) {
        self.behavior.smart_magnify(
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
            device_id,
        );
    }

    fn touchpad_rotate(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
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
            device_id,
            delta,
            phase,
        );
    }

    fn event(
        &mut self,
        window: &mut RunningWindow<CreateSurfaceRequest<User>>,
        event: <CreateSurfaceRequest<User> as Message>::Window,
    ) {
        self.behavior.event(
            event,
            Window::new(
                window,
                self.last_render.elapsed(),
                self.last_render_duration,
            ),
        );
    }
}

impl<T> UnwindSafe for KludgineWindow<T> {}

struct CallbackWindow<C> {
    callback: C,
    rendering: Drawing,
    keep_running: bool,
}

impl<T> WindowBehavior for CallbackWindow<T>
where
    T: for<'render, 'gfx, 'window> FnMut(Renderer<'render, 'gfx>, Window<'window>) -> bool
        + Send
        + UnwindSafe
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
        self.rendering.render(graphics);
        self.keep_running
    }
}

/// Runs a callback as a single window. Continues to run until false is
/// returned.
///
/// # Errors
///
/// Returns an [`EventLoopError`] upon the loop exiting due to an error. See
/// [`EventLoop::run`] for more information.
pub fn run<RenderFn>(render_fn: RenderFn) -> Result<(), EventLoopError>
where
    RenderFn: for<'render, 'gfx, 'window> FnMut(Renderer<'render, 'gfx>, Window<'window>) -> bool
        + Send
        + UnwindSafe
        + 'static,
{
    CallbackWindow::run_with(render_fn)
}

/// A handle to a window.
///
/// This handle does not prevent the window from being closed.
#[derive(Debug, Clone)]
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
