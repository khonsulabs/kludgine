use std::mem::size_of;
use std::panic::UnwindSafe;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use appit::winit::window::WindowId;
use appit::{Application, Message, RunningWindow, WindowBehavior as _};
use figures::units::UPx;
use figures::utils::lossy_f64_to_f32;
use figures::Size;

use crate::pipeline::PushConstants;
use crate::render::{Renderer, Rendering};
use crate::{Color, Graphics, Kludgine, RenderingGraphics};

fn shared_wgpu() -> Arc<wgpu::Instance> {
    static SHARED_WGPU: OnceLock<Arc<wgpu::Instance>> = OnceLock::new();
    SHARED_WGPU.get_or_init(Arc::default).clone()
}

/// An open window.
pub struct Window<'window> {
    window: &'window mut RunningWindow<CreateSurfaceRequest>,
    elapsed: Duration,
}

impl<'window> Window<'window> {
    fn new(window: &'window mut RunningWindow<CreateSurfaceRequest>, elapsed: Duration) -> Self {
        Self { window, elapsed }
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

    /// Returns the inner size of the window.
    #[must_use]
    pub fn inner_size(&self) -> Size<UPx> {
        self.window.inner_size().into()
    }

    /// Returns the current display scale factor.
    #[must_use]
    pub fn scale(&self) -> f32 {
        lossy_f64_to_f32(self.window.scale())
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
}

/// The behavior of a window.
pub trait WindowBehavior: Sized + 'static {
    /// The type of value provided during [`initialize()`](Self::initialize).
    ///
    /// In Kludgine, each window runs in its own thread. Kludgine allows does
    /// not require `WindowBehavior` implementors to implement `Send`, but it
    /// can be useful to receive input from the thread that is opening the
    /// window. This is where `Context` is useful: it must implement `Send`,
    /// allowing some data to still be passed when opening the window.
    type Context: UnwindSafe + Send + 'static;

    /// Initialize a new instance from the provided context.
    fn initialize(window: Window<'_>, graphics: &mut Graphics<'_>, context: Self::Context) -> Self;

    /// Prepare the window to render.
    ///
    /// This is called directly before [`render()`](Self::render()) and is a
    /// perfect place to update any prepared graphics as needed.
    #[allow(unused_variables)]
    fn prepare(&mut self, window: Window<'_>, graphics: &mut Graphics<'_>) {}

    /// Render the contents of the window.
    // TODO refactor away from bool return.
    fn render<'pass>(
        &'pass mut self,
        window: Window<'_>,
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
    fn run() -> !
    where
        Self::Context: Default,
    {
        KludgineWindow::<Self>::run_with_event_callback(create_surface)
    }

    /// Launches a Kludgine app using this window as the primary window.
    ///
    /// The `context` is passed along to [`initialize()`](Self::initialize) once
    /// the thread it is running on is spawned.
    fn run_with(context: Self::Context) -> ! {
        KludgineWindow::<Self>::run_with_context_and_event_callback(context, create_surface)
    }
}

#[allow(unsafe_code, clippy::needless_pass_by_value)]
fn create_surface(
    request: CreateSurfaceRequest,
    windows: &appit::Windows<CreateSurfaceRequest>,
) -> wgpu::Surface {
    let window = windows.get(request.0).expect("window not found");
    unsafe {
        shared_wgpu()
            .create_surface(&*window)
            .expect("error creating surface")
    }
}

struct CreateSurfaceRequest(WindowId);

impl Message for CreateSurfaceRequest {
    type Response = wgpu::Surface;
}

struct KludgineWindow<Behavior> {
    behavior: Behavior,
    kludgine: Kludgine,
    last_render: Instant,

    config: wgpu::SurfaceConfiguration,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    _adapter: wgpu::Adapter,
    wgpu: Arc<wgpu::Instance>,
}

impl<T> appit::WindowBehavior<CreateSurfaceRequest> for KludgineWindow<T>
where
    T: WindowBehavior + 'static,
{
    type Context = T::Context;

    #[allow(unsafe_code)]
    fn initialize(
        window: &mut RunningWindow<CreateSurfaceRequest>,
        context: Self::Context,
    ) -> Self {
        // SAFETY: This function is only invoked once the window has been
        // created, and cannot be invoked after the underlying window has been
        // destroyed.
        let surface = window
            .send(CreateSurfaceRequest(window.winit().id()))
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
                elapsed: Duration::from_secs(0),
            },
            &mut graphics,
            context,
        );

        Self {
            wgpu,
            kludgine: state,
            last_render,
            _adapter: adapter,
            behavior,
            config,
            surface,
            device,
            queue,
        }
    }

    #[allow(unsafe_code)]
    fn redraw(&mut self, window: &mut RunningWindow<CreateSurfaceRequest>) {
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
            Window::new(window, elapsed),
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
        self.behavior.render(Window::new(window, elapsed), &mut gfx);
        drop(gfx);
        frame.submit(&self.queue);
        surface.present();
        self.last_render = render_start;
    }

    fn close_requested(&mut self, _window: &mut RunningWindow<CreateSurfaceRequest>) -> bool {
        true
    }

    fn focus_changed(&mut self, _window: &mut RunningWindow<CreateSurfaceRequest>) {}

    fn occlusion_changed(&mut self, _window: &mut RunningWindow<CreateSurfaceRequest>) {}

    fn resized(&mut self, window: &mut RunningWindow<CreateSurfaceRequest>) {
        self.config.width = window.inner_size().width;
        self.config.height = window.inner_size().height;
        self.surface.configure(&self.device, &self.config);
        self.kludgine.resize(
            Size::new(window.inner_size().width, window.inner_size().height),
            lossy_f64_to_f32(window.scale()),
            &self.queue,
        );
        // TODO pass onto kludgine
        window.set_needs_redraw();
    }
}

impl<T> UnwindSafe for KludgineWindow<T> {}

struct CallbackWindow<C> {
    callback: C,
    rendering: Rendering,
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
            rendering: Rendering::default(),
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
pub fn run<RenderFn>(render_fn: RenderFn) -> !
where
    RenderFn: for<'render, 'gfx, 'window> FnMut(Renderer<'render, 'gfx>, Window<'window>) -> bool
        + Send
        + UnwindSafe
        + 'static,
{
    CallbackWindow::run_with(render_fn)
}
