use std::mem::size_of;
use std::sync::{Arc, OnceLock};

use appit::{RunningWindow, WindowBehavior as _};

use crate::shapes::PushConstants;
use crate::{Color, Graphics, Kludgine, Renderer, Rendering, RenderingGraphics};

fn shared_wgpu() -> Arc<wgpu::Instance> {
    static SHARED_WGPU: OnceLock<Arc<wgpu::Instance>> = OnceLock::new();
    SHARED_WGPU.get_or_init(Arc::default).clone()
}

pub trait WindowBehavior: Sized + 'static {
    type Context: Send + 'static;
    fn initialize(
        window: &mut RunningWindow,
        graphics: &mut Graphics<'_>,
        context: Self::Context,
    ) -> Self;

    #[allow(unused_variables)]
    fn prepare(&mut self, window: &mut RunningWindow, graphics: &mut Graphics<'_>) {}

    fn render<'pass>(
        &'pass mut self,
        window: &mut RunningWindow,
        graphics: &mut RenderingGraphics<'_, 'pass>,
    ) -> bool;

    fn power_preference() -> wgpu::PowerPreference {
        wgpu::PowerPreference::default()
    }

    fn limits(adapter_limits: wgpu::Limits) -> wgpu::Limits {
        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter_limits)
    }

    fn clear_color() -> Option<Color> {
        Some(Color::BLACK)
    }

    fn run() -> !
    where
        Self::Context: Default,
    {
        KludgineWindow::<Self>::run()
    }

    fn run_with(context: Self::Context) -> ! {
        KludgineWindow::<Self>::run_with((shared_wgpu(), context))
    }
}

struct KludgineWindow<Behavior> {
    behavior: Behavior,
    kludgine: Kludgine,

    config: wgpu::SurfaceConfiguration,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    _adapter: wgpu::Adapter,
    _wgpu: Arc<wgpu::Instance>,
}

impl<T> appit::WindowBehavior for KludgineWindow<T>
where
    T: WindowBehavior + 'static,
{
    type Context = (Arc<wgpu::Instance>, T::Context);

    fn initialize(window: &mut RunningWindow, (wgpu, context): Self::Context) -> Self {
        let surface = unsafe { wgpu.create_surface(window.winit()).unwrap() };
        let adapter = pollster::block_on(wgpu.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: T::power_preference(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .unwrap();
        let mut limits = T::limits(adapter.limits());
        limits.max_push_constant_size = size_of::<PushConstants>() as u32;
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
            window.inner_size().into(),
            window.scale() as f32,
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

        let behavior = T::initialize(window, &mut graphics, context);

        Self {
            _wgpu: wgpu,
            kludgine: state,
            _adapter: adapter,
            behavior,
            config,
            surface,
            device,
            queue,
        }
    }

    fn redraw(&mut self, window: &mut RunningWindow) {
        let frame = loop {
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
                        println!("Lost surface, reconfiguring");
                        self.surface =
                            unsafe { self._wgpu.create_surface(window.winit()).unwrap() };
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

        self.behavior.prepare(
            window,
            &mut Graphics::new(&mut self.kludgine, &self.device, &self.queue),
        );

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        });
        let mut gfx = RenderingGraphics::new(&mut pass, &self.kludgine, &self.device, &self.queue);
        self.behavior.render(window, &mut gfx);
        drop(pass);
        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }

    fn close_requested(&mut self, _window: &mut RunningWindow) -> bool {
        true
    }

    fn focus_changed(&mut self, _window: &mut RunningWindow) {}

    fn occlusion_changed(&mut self, _window: &mut RunningWindow) {}

    fn resized(&mut self, window: &mut RunningWindow) {
        self.config.width = window.inner_size().width;
        self.config.height = window.inner_size().height;
        self.surface.configure(&self.device, &self.config);
        self.kludgine.resize(
            window.inner_size().into(),
            window.scale() as f32,
            &self.queue,
        );
        // TODO pass onto kludgine
        window.set_needs_redraw();
    }
}

struct CallbackWindow<C> {
    callback: C,
    rendering: Rendering,
    keep_running: bool,
}

impl<T> WindowBehavior for CallbackWindow<T>
where
    T: for<'render, 'gfx> FnMut(Renderer<'render, 'gfx>, &mut RunningWindow) -> bool
        + Send
        + 'static,
{
    type Context = T;

    fn initialize(
        _window: &mut RunningWindow,
        _graphics: &mut Graphics<'_>,
        context: Self::Context,
    ) -> Self {
        Self {
            callback: context,
            rendering: Rendering::default(),
            keep_running: true,
        }
    }

    fn prepare(&mut self, window: &mut RunningWindow, graphics: &mut Graphics<'_>) {
        let renderer = self.rendering.new_frame(graphics);
        self.keep_running = (self.callback)(renderer, window);
    }

    fn render<'pass>(
        &'pass mut self,
        _window: &mut RunningWindow,
        graphics: &mut RenderingGraphics<'_, 'pass>,
    ) -> bool {
        self.rendering.render(graphics);
        self.keep_running
    }
}

pub fn run<RenderFn>(render_fn: RenderFn) -> !
where
    RenderFn: for<'render, 'gfx> FnMut(Renderer<'render, 'gfx>, &mut RunningWindow) -> bool
        + Send
        + 'static,
{
    CallbackWindow::run_with(render_fn)
}
