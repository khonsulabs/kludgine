use std::{
    collections::HashMap,
    num::NonZeroU32,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use easygpu::{
    prelude::*,
    wgpu::{
        Buffer, Extent3d, FilterMode, Origin3d, PresentMode, TextureAspect, TextureUsages,
        COPY_BYTES_PER_ROW_ALIGNMENT,
    },
};
use easygpu_lyon::LyonPipeline;
use figures::Rectlike;
use futures::FutureExt;
use image::DynamicImage;
use instant::Duration;

use crate::{
    delay,
    math::{ExtentsRect, Point, Size, Unknown},
    scene::SceneEvent,
    sprite::{self, VertexShaderSource},
};

mod frame;
use frame::{FontUpdate, Frame, FrameCommand};

/// Renders frames created by a [`Scene`](crate::scene::Scene).
pub struct FrameRenderer<T>
where
    T: VertexShaderSource,
{
    keep_running: Arc<AtomicBool>,
    shutdown: Option<Box<dyn ShutdownCallback>>,
    renderer: Renderer,
    multisample_texture: Option<Texture>,
    destination: Destination,
    sprite_pipeline: sprite::Pipeline<T>,
    shape_pipeline: LyonPipeline<T::Lyon>,
    gpu_state: Mutex<GpuState>,
    scene_event_receiver: flume::Receiver<SceneEvent>,
}

#[derive(Default)]
struct GpuState {
    textures: HashMap<u64, BindingGroup>,
}

#[allow(clippy::large_enum_variant)]
enum Destination {
    Uninitialized,
    Device,
    Texture {
        color: Texture,
        depth: DepthBuffer,
        output: Buffer,
    },
}

enum Output<'a> {
    SwapChain(RenderFrame),
    Texture {
        color: &'a Texture,
        depth: &'a DepthBuffer,
    },
}

impl<'a> RenderTarget for Output<'a> {
    fn color_target(&self) -> &wgpu::TextureView {
        match self {
            Output::SwapChain(swap) => swap.color_target(),
            Output::Texture { color, .. } => &color.view,
        }
    }

    fn zdepth_target(&self) -> &wgpu::TextureView {
        match self {
            Output::SwapChain(swap) => swap.zdepth_target(),
            Output::Texture { depth, .. } => &depth.texture.view,
        }
    }
}

enum RenderCommand {
    SpriteBuffer(u64, sprite::BatchBuffers),
    FontBuffer(u64, sprite::BatchBuffers),
    Shapes(easygpu_lyon::Shape),
}

impl<T> FrameRenderer<T>
where
    T: VertexShaderSource + Send + Sync + 'static,
{
    fn new(
        renderer: Renderer,
        keep_running: Arc<AtomicBool>,
        scene_event_receiver: flume::Receiver<SceneEvent>,
    ) -> Self {
        let shape_pipeline = renderer.pipeline(Blending::default(), T::sampler_format());
        let sprite_pipeline = renderer.pipeline(Blending::default(), T::sampler_format());
        Self {
            renderer,
            keep_running,
            destination: Destination::Uninitialized,
            sprite_pipeline,
            shape_pipeline,
            scene_event_receiver,
            shutdown: None,
            multisample_texture: None,
            gpu_state: Mutex::new(GpuState::default()),
        }
    }

    /// Launches a thread that renders the results of the `SceneEvent`s.
    pub fn run<F: ShutdownCallback>(
        renderer: Renderer,
        keep_running: Arc<AtomicBool>,
        scene_event_receiver: flume::Receiver<SceneEvent>,
        shutdown_callback: F,
    ) {
        let mut frame_renderer = Self::new(renderer, keep_running, scene_event_receiver);
        frame_renderer.shutdown = Some(Box::new(shutdown_callback));
        std::thread::Builder::new()
            .name(String::from("kludgine-frame-renderer"))
            .spawn(move || frame_renderer.render_loop())
            .unwrap();
    }

    /// Launches a thread that renders the results of the `SceneEvent`s.
    pub async fn render_one_frame(
        renderer: Renderer,
        scene_event_receiver: flume::Receiver<SceneEvent>,
    ) -> crate::Result<DynamicImage> {
        let mut frame_renderer = Self::new(renderer, Arc::default(), scene_event_receiver);
        let mut frame = Frame::default();
        let _ = frame.update(&frame_renderer.scene_event_receiver);
        frame_renderer.render_frame(&mut frame)?;
        if let Destination::Texture { output, .. } = frame_renderer.destination {
            let data = output.slice(..);
            let mut map_async = Box::pin(data.map_async(wgpu::MapMode::Read).fuse());
            let wgpu_device = frame_renderer.renderer.device.wgpu;
            let mut poll_loop = Box::pin(
                async move {
                    loop {
                        wgpu_device.poll(wgpu::Maintain::Poll);
                        delay::Delay::new(Duration::from_millis(1)).await;
                    }
                }
                .fuse(),
            );
            while futures::select! {
                _ = map_async => false,
                _ = poll_loop => true,
            } {}

            let bytes = data.get_mapped_range().to_vec();

            let frame_size = frame.size.cast::<usize>();
            let bytes_per_row = size_for_aligned_copy(frame_size.width * 4);
            Ok(image::DynamicImage::ImageBgra8(
                if bytes_per_row == frame_size.width * 4 {
                    image::ImageBuffer::from_vec(
                        frame_size.width as u32,
                        frame_size.height as u32,
                        bytes,
                    )
                    .unwrap()
                } else {
                    image::ImageBuffer::from_fn(
                        frame_size.width as u32,
                        frame_size.height as u32,
                        move |x, y| {
                            let offset = y as usize * bytes_per_row + x as usize * 4;
                            image::Bgra([
                                bytes[offset],
                                bytes[offset + 1],
                                bytes[offset + 2],
                                bytes[offset + 3],
                            ])
                        },
                    )
                },
            ))
        } else {
            panic!("render_one_frame only works with an offscreen renderer")
        }
    }

    fn render_loop(mut self) {
        let mut frame = Frame::default();
        loop {
            if !self.keep_running.load(Ordering::SeqCst) {
                let shutdown = self.shutdown.take();
                // These drops prevents a segfault on exit per. The shutdown method must be
                // called after self is dropped. https://github.com/gfx-rs/wgpu/issues/1439
                drop(self);
                drop(frame);
                if let Some(mut shutdown) = shutdown {
                    shutdown.shutdown();
                }
                return;
            }
            if frame.update(&self.scene_event_receiver) {
                self.render_frame(&mut frame)
                    .expect("Error rendering window");
            } else {
                self.keep_running.store(false, Ordering::SeqCst);
            }
        }
    }

    fn create_destination(
        renderer: &mut Renderer,
        frame_size: Size<u32, ScreenSpace>,
    ) -> Destination {
        if renderer.device.surface.is_some() {
            renderer.configure(frame_size, PresentMode::Fifo, T::sampler_format());
            Destination::Device
        } else {
            let color = renderer.texture(
                frame_size,
                T::sampler_format(),
                TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::COPY_SRC
                    | TextureUsages::RENDER_ATTACHMENT,
                false,
            );
            let depth = renderer
                .device
                .create_zbuffer(frame_size, renderer.sample_count());
            let output = renderer.device.wgpu.create_buffer(&wgpu::BufferDescriptor {
                label: Some("output buffer"),
                size: buffer_size(frame_size) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            Destination::Texture {
                color,
                depth,
                output,
            }
        }
    }

    #[allow(clippy::too_many_lines)] // TODO refactor
    fn render_frame(&mut self, engine_frame: &mut Frame) -> crate::Result<()> {
        let frame_size = engine_frame.size.cast::<u32>();
        if frame_size.width == 0 || frame_size.height == 0 {
            return Ok(());
        }

        let output = match &mut self.destination {
            Destination::Uninitialized => {
                self.destination = Self::create_destination(&mut self.renderer, frame_size);
                return self.render_frame(engine_frame);
            }
            Destination::Device => {
                if self.renderer.device.size() != frame_size {
                    self.renderer
                        .configure(frame_size, PresentMode::Fifo, T::sampler_format());
                }

                let output = match self.renderer.current_frame() {
                    Ok(texture) => texture,
                    Err(wgpu::SurfaceError::Outdated) => return Ok(()), /* Ignore outdated,
                                                                            * we'll draw */
                    // next time.
                    Err(err) => panic!("Unrecoverable error on swap chain {:?}", err),
                };
                Output::SwapChain(output)
            }
            Destination::Texture {
                color,
                depth,
                output,
            } => {
                if color.size != frame_size {
                    if let Destination::Texture {
                        color: new_color,
                        depth: new_depth,
                        output: new_output,
                    } = Self::create_destination(&mut self.renderer, frame_size)
                    {
                        *color = new_color;
                        *depth = new_depth;
                        *output = new_output;
                    } else {
                        unreachable!();
                    }
                }
                Output::Texture { color, depth }
            }
        };
        let mut frame = self.renderer.frame();

        let ortho = ScreenTransformation::ortho(
            0.,
            0.,
            frame_size.width as f32,
            frame_size.height as f32,
            -1.,
            1.,
        );
        self.renderer.update_pipeline(&self.shape_pipeline, ortho);

        self.renderer.update_pipeline(&self.sprite_pipeline, ortho);

        {
            let mut render_commands = Vec::new();
            let mut gpu_state = self
                .gpu_state
                .try_lock()
                .expect("There should be no contention");

            for FontUpdate {
                font_id,
                rect,
                data,
            } in &engine_frame.pending_font_updates
            {
                let mut loaded_font = engine_frame.fonts.get_mut(font_id).unwrap();
                if loaded_font.texture.is_none() {
                    let texture = self.renderer.texture(
                        Size::new(512, 512),
                        T::texture_format(),
                        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                        false,
                    ); // TODO font texture should be configurable
                    let sampler = self
                        .renderer
                        .sampler(FilterMode::Linear, FilterMode::Linear);

                    let binding = self
                        .sprite_pipeline
                        .binding(&self.renderer, &texture, &sampler);
                    loaded_font.binding = Some(binding);
                    loaded_font.texture = Some(texture);
                }

                let row_bytes = size_for_aligned_copy(rect.width() as usize * 4);
                let mut pixels = Vec::with_capacity(row_bytes * rect.height() as usize);
                let mut pixel_iterator = data.iter();
                for _ in 0..rect.height() {
                    for _ in 0..rect.width() {
                        let p = pixel_iterator.next().unwrap();
                        pixels.push(255);
                        pixels.push(255);
                        pixels.push(255);
                        pixels.push(*p);
                    }

                    pixels.resize_with(size_for_aligned_copy(pixels.len()), Default::default);
                }

                let pixels = Rgba8::align(&pixels);
                self.renderer.submit(&[Op::Transfer {
                    f: loaded_font.texture.as_ref().unwrap(),
                    buf: pixels,
                    rect: ExtentsRect::new(
                        Point::new(rect.min.x, rect.min.y),
                        Point::new(rect.max.x, rect.max.y),
                    )
                    .as_sized()
                    .cast::<i32>(),
                }]);
            }
            engine_frame.pending_font_updates.clear();

            for command in std::mem::take(&mut engine_frame.commands) {
                match command {
                    FrameCommand::LoadTexture(texture) => {
                        if !gpu_state.textures.contains_key(&texture.id()) {
                            let sampler = self
                                .renderer
                                .sampler(FilterMode::Nearest, FilterMode::Nearest);

                            let (gpu_texture, texels, texture_id) = {
                                let (w, h) = texture.image.dimensions();
                                let bytes_per_row = size_for_aligned_copy(w as usize * 4);
                                let mut pixels = Vec::with_capacity(bytes_per_row * h as usize);
                                for (_, row) in texture.image.enumerate_rows() {
                                    for (_, _, pixel) in row {
                                        pixels.push(pixel[0]);
                                        pixels.push(pixel[1]);
                                        pixels.push(pixel[2]);
                                        pixels.push(pixel[3]);
                                    }

                                    pixels.resize_with(
                                        size_for_aligned_copy(pixels.len()),
                                        Default::default,
                                    );
                                }
                                let pixels = Rgba8::align(&pixels);

                                (
                                    self.renderer.texture(
                                        Size::new(w, h).cast::<u32>(),
                                        T::texture_format(),
                                        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                                        false,
                                    ),
                                    pixels.to_owned(),
                                    texture.id(),
                                )
                            };

                            self.renderer
                                .submit(&[Op::Fill(&gpu_texture, texels.as_slice())]);

                            gpu_state.textures.insert(
                                texture_id,
                                self.sprite_pipeline.binding(
                                    &self.renderer,
                                    &gpu_texture,
                                    &sampler,
                                ),
                            );
                        }
                    }
                    FrameCommand::DrawBatch(batch) => {
                        let mut gpu_batch = sprite::GpuBatch::new(
                            batch.size.cast_unit(),
                            batch.clipping_rect.map(|r| r.as_extents()),
                        );
                        for sprite_handle in &batch.sprites {
                            gpu_batch.add_sprite(sprite_handle.clone());
                        }
                        render_commands.push(RenderCommand::SpriteBuffer(
                            batch.loaded_texture_id,
                            gpu_batch.finish(&self.renderer),
                        ));
                    }
                    FrameCommand::DrawShapes(batch) => {
                        render_commands.push(RenderCommand::Shapes(batch.finish(&self.renderer)?));
                        // let prepared_shape = batch.finish(&self.renderer)?;
                        // pass.set_easy_pipeline(&self.shape_pipeline);
                        // prepared_shape.draw(&mut pass);
                    }
                    FrameCommand::DrawText { text, clip } => {
                        if let Some(loaded_font) = engine_frame.fonts.get(&text.font.id()) {
                            if let Some(texture) = loaded_font.texture.as_ref() {
                                let mut batch = sprite::GpuBatch::new(
                                    texture.size,
                                    clip.map(|r| r.as_extents()),
                                );
                                for (uv_rect, screen_rect) in text.glyphs.iter().filter_map(|g| {
                                    loaded_font.cache.rect_for(0, &g.glyph).ok().flatten()
                                }) {
                                    // This is one section that feels like a kludge. gpu_cache is
                                    // storing the textures upside down like normal but easywgpu is
                                    // automatically flipping textures. Easygpu's texture isn't
                                    // exactly the best compatibility with this process
                                    // because gpu_cache also produces data that is 1 byte per
                                    // pixel, and we have to expand it when we're updating the
                                    // texture
                                    let source = ExtentsRect::<_, Unknown>::new(
                                        Point::new(
                                            uv_rect.min.x * 512.0,
                                            (1.0 - uv_rect.max.y) * 512.0,
                                        ),
                                        Point::new(
                                            uv_rect.max.x * 512.0,
                                            (1.0 - uv_rect.min.y) * 512.0,
                                        ),
                                    );

                                    let dest = ExtentsRect::new(
                                        text.location
                                            + figures::Vector::new(
                                                screen_rect.min.x as f32,
                                                screen_rect.min.y as f32,
                                            ),
                                        text.location
                                            + figures::Vector::new(
                                                screen_rect.max.x as f32,
                                                screen_rect.max.y as f32,
                                            ),
                                    );
                                    batch.add_box(
                                        source.cast_unit().cast(),
                                        dest,
                                        sprite::SpriteRotation::none(),
                                        text.color.into(),
                                    );
                                }
                                render_commands.push(RenderCommand::FontBuffer(
                                    loaded_font.font.id(),
                                    batch.finish(&self.renderer),
                                ));
                            }
                        }
                    }
                }
            }
            if self
                .multisample_texture
                .as_ref()
                .map_or(true, |t| t.size != frame_size)
            {
                self.multisample_texture = Some(self.renderer.texture(
                    frame_size,
                    T::sampler_format(),
                    TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    true,
                ));
            }
            let mut pass = frame.pass(
                PassOp::Clear(Rgba::TRANSPARENT),
                &output,
                Some(&self.multisample_texture.as_ref().unwrap().view),
            );
            for command in &render_commands {
                match command {
                    RenderCommand::SpriteBuffer(texture_id, buffer) => {
                        pass.set_easy_pipeline(&self.sprite_pipeline);
                        let binding = gpu_state.textures.get(texture_id).unwrap();
                        pass.easy_draw(buffer, binding);
                    }
                    RenderCommand::FontBuffer(font_id, buffer) => {
                        pass.set_easy_pipeline(&self.sprite_pipeline);
                        if let Some(binding) = engine_frame
                            .fonts
                            .get(font_id)
                            .and_then(|f| f.binding.as_ref())
                        {
                            pass.easy_draw(buffer, binding);
                        }
                    }
                    RenderCommand::Shapes(shapes) => {
                        pass.set_easy_pipeline(&self.shape_pipeline);
                        shapes.draw(&mut pass);
                    }
                }
            }
        }

        if let Destination::Texture { output, color, .. } = &self.destination {
            frame.encoder_mut().copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture: &color.wgpu,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                wgpu::ImageCopyBuffer {
                    buffer: output,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: NonZeroU32::new(size_for_aligned_copy(
                            frame_size.width as usize * 4,
                        ) as u32),
                        rows_per_image: NonZeroU32::new(frame_size.height as u32),
                    },
                },
                Extent3d {
                    width: frame_size.width,
                    height: frame_size.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        self.renderer.present(frame);

        Ok(())
    }
}

const fn size_for_aligned_copy(bytes: usize) -> usize {
    let chunks =
        (bytes + COPY_BYTES_PER_ROW_ALIGNMENT as usize - 1) / COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    chunks * COPY_BYTES_PER_ROW_ALIGNMENT as usize
}

const fn buffer_size(size: Size<u32, ScreenSpace>) -> usize {
    size_for_aligned_copy(size.width as usize * 4) * size.height as usize
}

/// A callback that can be invoked when a [`FrameRenderer`] is fully shut down.
pub trait ShutdownCallback: Send + Sync + 'static {
    /// Invoked when the [`FrameRenderer`] is fully shut down.
    fn shutdown(&mut self);
}

impl<F> ShutdownCallback for F
where
    F: FnMut() + Send + Sync + 'static,
{
    fn shutdown(&mut self) {
        self();
    }
}
