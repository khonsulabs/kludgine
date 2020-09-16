use crate::{
    math::{Point, Unknown},
    runtime::Runtime,
    sprite,
    window::frame::{FontUpdate, Frame, FrameCommand},
    KludgineResult,
};
use crossbeam::atomic::AtomicCell;
use euclid::Box2D;
use rgx::{
    color::{Rgba, Rgba8},
    core,
};
use rgx_lyon::LyonPipeline;
use std::sync::Arc;

pub(crate) struct FrameSynchronizer {
    receiver: async_channel::Receiver<Frame>,
    sender: async_channel::Sender<Frame>,
}

impl FrameSynchronizer {
    pub fn pair() -> (FrameSynchronizer, FrameSynchronizer) {
        let (a_sender, a_receiver) = async_channel::bounded(1);
        let (b_sender, b_receiver) = async_channel::bounded(1);

        let a_synchronizer = FrameSynchronizer {
            sender: b_sender,
            receiver: a_receiver,
        };
        let b_synchronizer = FrameSynchronizer {
            sender: a_sender,
            receiver: b_receiver,
        };

        (a_synchronizer, b_synchronizer)
    }

    pub async fn take(&mut self) -> Frame {
        // Ignoring the error because if the sender/receiver is disconnected
        // the window is closing and we should just ignore the error and let
        // it close.
        self.receiver.recv().await.unwrap_or_default()
    }

    pub async fn relinquish(&mut self, frame: Frame) {
        // Ignoring the error because if the sender/receiver is disconnected
        // the window is closing and we should just ignore the error and let
        // it close.
        self.sender.send(frame).await.unwrap_or_default();
    }
}

pub(crate) struct FrameRenderer {
    keep_running: Arc<AtomicCell<bool>>,
    renderer: core::Renderer,
    swap_chain: core::SwapChain,
    frame_synchronizer: FrameSynchronizer,
    sprite_pipeline: sprite::Pipeline,
    shape_pipeline: LyonPipeline,
}

impl FrameRenderer {
    fn new(
        renderer: core::Renderer,
        frame_synchronizer: FrameSynchronizer,
        keep_running: Arc<AtomicCell<bool>>,
        initial_width: u32,
        initial_height: u32,
    ) -> Self {
        let swap_chain =
            renderer.swap_chain(initial_width, initial_height, core::PresentMode::Vsync);
        let shape_pipeline = renderer.pipeline(core::Blending::default());
        let sprite_pipeline = renderer.pipeline(core::Blending::default());
        Self {
            renderer,
            keep_running,
            swap_chain,
            frame_synchronizer,
            sprite_pipeline,
            shape_pipeline,
        }
    }
    pub fn run(
        renderer: core::Renderer,
        keep_running: Arc<AtomicCell<bool>>,
        initial_width: u32,
        initial_height: u32,
    ) -> FrameSynchronizer {
        let (client_synchronizer, renderer_synchronizer) = FrameSynchronizer::pair();

        let frame_renderer = FrameRenderer::new(
            renderer,
            renderer_synchronizer,
            keep_running,
            initial_width,
            initial_height,
        );
        Runtime::spawn(frame_renderer.render_loop()).detach();

        client_synchronizer
    }

    async fn render_loop(mut self) {
        loop {
            if !self.keep_running.load() {
                return;
            }
            self.render().await.expect("Error rendering window");
        }
    }

    pub async fn render(&mut self) -> KludgineResult<()> {
        let mut engine_frame = self.frame_synchronizer.take().await;
        let result = self.render_frame(&mut engine_frame).await;
        self.frame_synchronizer.relinquish(engine_frame).await;
        result
    }

    async fn render_frame(&mut self, engine_frame: &mut Frame) -> KludgineResult<()> {
        let (w, h) = {
            (
                engine_frame.size.width as u32,
                engine_frame.size.height as u32,
            )
        };
        if w == 0 || h == 0 {
            return Ok(());
        }

        for FontUpdate { font, rect, data } in engine_frame.pending_font_updates.iter() {
            let mut loaded_font = font.handle.write().await;
            if loaded_font.texture.is_none() {
                let texture = self.renderer.texture(512, 512); // TODO font texture should be configurable
                let sampler = self
                    .renderer
                    .sampler(core::Filter::Nearest, core::Filter::Nearest);

                let binding = self
                    .sprite_pipeline
                    .binding(&self.renderer, &texture, &sampler);
                loaded_font.binding = Some(binding);
                loaded_font.texture = Some(texture);
            }
            let mut pixels = Vec::with_capacity(data.len() * 4);
            for p in data.iter() {
                pixels.push(*p);
                pixels.push(*p);
                pixels.push(*p);
                pixels.push(*p);
            }
            let pixels = Rgba8::align(&pixels);

            self.renderer.submit(&[core::Op::Transfer(
                loaded_font.texture.as_ref().unwrap(),
                pixels,
                rect.width(),
                rect.height(),
                rgx::rect::Rect::new(
                    rect.min.x as i32,
                    rect.min.y as i32,
                    rect.max.x as i32,
                    rect.max.y as i32,
                ),
            )]);
        }
        engine_frame.pending_font_updates.clear();

        if self.swap_chain.width != w || self.swap_chain.height != h {
            self.swap_chain = self.renderer.swap_chain(w, h, core::PresentMode::Vsync);
        }

        let output = self.swap_chain.next();
        let mut frame = self.renderer.frame();

        let ortho = ortho(output.width, output.height, Default::default());
        self.renderer
            .update_pipeline(&self.shape_pipeline, ortho, &mut frame);

        self.renderer
            .update_pipeline(&self.sprite_pipeline, ortho, &mut frame);

        {
            let mut pass = frame.pass(core::PassOp::Clear(Rgba::TRANSPARENT), &output);
            for command in std::mem::take(&mut engine_frame.commands) {
                match command {
                    FrameCommand::LoadTexture(texture_handle) => {
                        let mut loaded_texture = texture_handle.handle.write().await;
                        if loaded_texture.binding.is_none() {
                            let sampler = self
                                .renderer
                                .sampler(core::Filter::Nearest, core::Filter::Nearest);

                            let (gpu_texture, texels) = {
                                let texture = loaded_texture.texture.handle.read().await;
                                let (w, h) = texture.image.dimensions();
                                let pixels = texture.image.pixels().cloned().collect::<Vec<_>>();
                                let pixels = Rgba8::align(&pixels);

                                (self.renderer.texture(w as u32, h as u32), pixels.to_owned())
                            };

                            self.renderer
                                .submit(&[core::Op::Fill(&gpu_texture, texels.as_slice())]);

                            loaded_texture.binding = Some(self.sprite_pipeline.binding(
                                &self.renderer,
                                &gpu_texture,
                                &sampler,
                            ));
                        }
                    }
                    FrameCommand::DrawBatch(batch) => {
                        let loaded_texture = batch.loaded_texture.handle.read().await;
                        let texture = loaded_texture.texture.handle.read().await;

                        let mut gpu_batch =
                            sprite::GpuBatch::new(texture.image.width(), texture.image.height());
                        for sprite_handle in batch.sprites.iter() {
                            gpu_batch.add_sprite(sprite_handle.clone()).await;
                        }
                        let buffer = gpu_batch.finish(&self.renderer);

                        pass.set_pipeline(&self.sprite_pipeline);
                        pass.draw(
                            &buffer,
                            loaded_texture
                                .binding
                                .as_ref()
                                .expect("Empty binding on texture"),
                        );
                    }
                    FrameCommand::DrawShapes(batch) => {
                        let prepared_shape = batch.finish(&self.renderer)?;
                        pass.set_pipeline(&self.shape_pipeline);
                        prepared_shape.draw(&mut pass);
                    }
                    FrameCommand::DrawText { text, loaded_font } => {
                        let text_data = text.handle.read().await;
                        let loaded_font_data = loaded_font.handle.read().await;
                        if let Some(texture) = loaded_font_data.texture.as_ref() {
                            let mut batch = sprite::GpuBatch::new(texture.w, texture.h);
                            for (uv_rect, screen_rect) in
                                text_data.positioned_glyphs.iter().filter_map(|g| {
                                    loaded_font_data.cache.rect_for(0, g).ok().flatten()
                                })
                            {
                                // This is one section that feels like a kludge. gpu_cache is storing the textures upside down like normal
                                // but rgx is automatically flipping textures. Rgx isn't exactly the best compatibility with this process
                                // because gpu_cache also produces data that is 1 byte per pixel, and we have to expand it when we're updating the texture
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
                                    text_data.color.into(),
                                );
                            }
                            let buffer = batch.finish(&self.renderer);

                            pass.set_pipeline(&self.sprite_pipeline);
                            pass.draw(
                                &buffer,
                                loaded_font_data
                                    .binding
                                    .as_ref()
                                    .expect("Empty binding on texture"),
                            );
                        }
                    }
                }
            }
        }

        self.renderer.present(frame);

        Ok(())
    }
}

pub fn ortho(w: u32, h: u32, origin: rgx::kit::Origin) -> rgx::math::Matrix4<f32> {
    let (bottom, top) = match origin {
        rgx::kit::Origin::BottomLeft => (h as f32, 0.),
        rgx::kit::Origin::TopLeft => (0., h as f32),
    };
    rgx::math::Ortho::<f32> {
        left: 0.0,
        right: w as f32,
        bottom,
        top,
        near: -1.0,
        far: 1.0,
    }
    .into()
}
