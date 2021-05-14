use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crossbeam::atomic::AtomicCell;
use easygpu::{
    prelude::*,
    wgpu::{FilterMode, COPY_BYTES_PER_ROW_ALIGNMENT},
};
use easygpu_lyon::LyonPipeline;

use crate::{
    math::{Box2D, Point, Size, Unknown},
    runtime::RuntimeRequest,
    scene::SceneEvent,
    sprite::{self, VertexShaderSource},
    window::frame::{FontUpdate, Frame, FrameCommand},
    KludgineResult,
};

pub(crate) struct FrameRenderer<T>
where
    T: VertexShaderSource,
{
    keep_running: Arc<AtomicCell<bool>>,
    renderer: Renderer,
    swap_chain: SwapChain,
    sprite_pipeline: sprite::Pipeline<T>,
    shape_pipeline: LyonPipeline<T::Lyon>,
    gpu_state: Mutex<GpuState>,
    scene_event_receiver: flume::Receiver<SceneEvent>,
}

#[derive(Default)]
struct GpuState {
    textures: HashMap<u64, BindingGroup>,
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
        keep_running: Arc<AtomicCell<bool>>,
        scene_event_receiver: flume::Receiver<SceneEvent>,
        initial_size: Size<u32, ScreenSpace>,
    ) -> Self {
        let swap_chain = renderer.swap_chain(initial_size, PresentMode::Vsync, T::sampler_format());
        let shape_pipeline = renderer.pipeline(Blending::default(), T::sampler_format());
        let sprite_pipeline = renderer.pipeline(Blending::default(), T::sampler_format());
        Self {
            renderer,
            keep_running,
            swap_chain,
            sprite_pipeline,
            shape_pipeline,
            scene_event_receiver,
            gpu_state: Mutex::new(GpuState::default()),
        }
    }

    /// Launches a thread that renders the results of the window-loop
    /// generating frames.
    pub fn run(
        renderer: Renderer,
        keep_running: Arc<AtomicCell<bool>>,
        scene_event_receiver: flume::Receiver<SceneEvent>,
        initial_size: Size<u32, ScreenSpace>,
    ) {
        let frame_renderer =
            FrameRenderer::<T>::new(renderer, keep_running, scene_event_receiver, initial_size);
        std::thread::Builder::new()
            .name(String::from("kludgine-frame-renderer"))
            .spawn(move || frame_renderer.render_loop())
            .unwrap();
    }

    fn render_loop(mut self) {
        let mut frame = Frame::default();
        loop {
            if !self.keep_running.load() {
                // This drop prevents a segfault on exit per
                // https://github.com/gfx-rs/wgpu-rs/issues/911
                drop(self);
                RuntimeRequest::WindowClosed.send().unwrap();
                return;
            }
            frame.update(&self.scene_event_receiver);
            self.render_frame(&mut frame)
                .expect("Error rendering window");
        }
    }

    fn render_frame(&mut self, engine_frame: &mut Frame) -> KludgineResult<()> {
        let frame_size = engine_frame.size.cast::<u32>();
        if frame_size.width == 0 || frame_size.height == 0 {
            return Ok(());
        }

        if self.swap_chain.size != frame_size {
            self.swap_chain =
                self.renderer
                    .swap_chain(frame_size, PresentMode::Vsync, T::sampler_format());
        }

        let output = match self.swap_chain.next_texture() {
            Ok(texture) => texture,
            Err(wgpu::SwapChainError::Outdated) => return Ok(()), /* Ignore outdated, we'll draw */
            // next time.
            Err(err) => panic!("Unrecoverable error on swap chain {:?}", err),
        };
        let mut frame = self.renderer.frame();

        let ortho = ScreenTransformation::ortho(
            0.,
            output.size.width as f32,
            output.size.height as f32,
            0.,
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
            } in engine_frame.pending_font_updates.iter()
            {
                let mut loaded_font = engine_frame.fonts.get_mut(font_id).unwrap();
                if loaded_font.texture.is_none() {
                    let texture = self
                        .renderer
                        .texture(Size::new(512, 512), T::texture_format()); // TODO font texture should be configurable
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
                    rect: Box2D::new(
                        Point::new(rect.min.x, rect.min.y),
                        Point::new(rect.max.x, rect.max.y),
                    )
                    .to_rect()
                    .cast::<i32>(),
                }]);
            }
            engine_frame.pending_font_updates.clear();

            for command in std::mem::take(&mut engine_frame.commands) {
                match command {
                    FrameCommand::LoadTexture(texture) => {
                        if !gpu_state.textures.contains_key(&texture.id) {
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
                                    ),
                                    pixels.to_owned(),
                                    texture.id,
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
                            batch.clipping_rect.map(|r| r.to_box2d()),
                        );
                        for sprite_handle in batch.sprites.iter() {
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
                        if let Some(loaded_font) = engine_frame.fonts.get(&text.data.font.id()) {
                            if let Some(texture) = loaded_font.texture.as_ref() {
                                let mut batch =
                                    sprite::GpuBatch::new(texture.size, clip.map(|r| r.to_box2d()));
                                for (uv_rect, screen_rect) in
                                    text.data.glyphs.iter().filter_map(|g| {
                                        loaded_font.cache.rect_for(0, &g.glyph).ok().flatten()
                                    })
                                {
                                    // This is one section that feels like a kludge. gpu_cache is
                                    // storing the textures upside down like normal but easywgpu is
                                    // automatically flipping textures. Easygpu's texture isn't
                                    // exactly the best compatibility with this process
                                    // because gpu_cache also produces data that is 1 byte per
                                    // pixel, and we have to expand it when we're updating the
                                    // texture
                                    let source = Box2D::<_, Unknown>::new(
                                        Point::new(
                                            uv_rect.min.x * 512.0,
                                            (1.0 - uv_rect.max.y) * 512.0,
                                        ),
                                        Point::new(
                                            uv_rect.max.x * 512.0,
                                            (1.0 - uv_rect.min.y) * 512.0,
                                        ),
                                    );

                                    let dest = Box2D::new(
                                        text.location
                                            + euclid::Vector2D::new(
                                                screen_rect.min.x as f32,
                                                screen_rect.min.y as f32,
                                            ),
                                        text.location
                                            + euclid::Vector2D::new(
                                                screen_rect.max.x as f32,
                                                screen_rect.max.y as f32,
                                            ),
                                    );
                                    batch.add_box(
                                        source.cast_unit().cast(),
                                        dest,
                                        sprite::SpriteRotation::default(),
                                        text.data.color.into(),
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
            let mut pass = frame.pass(PassOp::Clear(Rgba::TRANSPARENT), &output);
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
                            .map(|f| f.binding.as_ref())
                            .flatten()
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

        self.renderer.present(frame);

        Ok(())
    }
}

fn size_for_aligned_copy(bytes: usize) -> usize {
    let chunks =
        (bytes + COPY_BYTES_PER_ROW_ALIGNMENT as usize - 1) / COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    chunks * COPY_BYTES_PER_ROW_ALIGNMENT as usize
}
