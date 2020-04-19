use crate::{
    frame::{FontUpdate, Frame, FrameCommand},
    runtime::{Runtime, FRAME_DURATION},
    timing::FrequencyLimiter,
    KludgineHandle, KludgineResult,
};
use crossbeam::atomic::AtomicCell;
use rgx::core::*;
use rgx::kit;
use rgx::kit::{shape2d, sprite2d, Repeat, ZDepth};
use std::{sync::Arc, time::Duration};

pub(crate) struct FrameRenderer {
    keep_running: Arc<AtomicCell<bool>>,
    renderer: Renderer,
    swap_chain: SwapChain,
    frame: KludgineHandle<Frame>,
    sprite_pipeline: sprite2d::Pipeline,
    shape_pipeline: shape2d::Pipeline,
}

impl FrameRenderer {
    fn new(
        renderer: Renderer,
        frame: KludgineHandle<Frame>,
        keep_running: Arc<AtomicCell<bool>>,
        initial_width: u32,
        initial_height: u32,
    ) -> Self {
        let swap_chain = renderer.swap_chain(initial_width, initial_height, PresentMode::NoVsync);
        let shape_pipeline = renderer.pipeline(Blending::default());
        let sprite_pipeline = renderer.pipeline(Blending::default());
        Self {
            renderer,
            keep_running,
            swap_chain,
            frame,
            sprite_pipeline,
            shape_pipeline,
        }
    }
    pub fn run(
        renderer: Renderer,
        frame: KludgineHandle<Frame>,
        keep_running: Arc<AtomicCell<bool>>,
        initial_width: u32,
        initial_height: u32,
    ) {
        let frame_renderer =
            FrameRenderer::new(renderer, frame, keep_running, initial_width, initial_height);
        Runtime::spawn(frame_renderer.render_loop());
    }

    async fn render_loop(mut self) {
        let mut limiter = FrequencyLimiter::new(Duration::from_nanos(FRAME_DURATION));
        loop {
            if let Some(remaining) = limiter.remaining() {
                if !self.keep_running.load() {
                    return;
                }

                async_std::task::sleep(remaining).await;
            }
            self.render().expect("Error rendering window");
            limiter.advance_frame();
        }
    }

    pub fn render(&mut self) -> KludgineResult<()> {
        let mut engine_frame = self.frame.write().expect("Error locking frame");
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
            let mut loaded_font = font
                .handle
                .write()
                .expect("Error locking font for updating GPU");
            if loaded_font.texture.is_none() {
                let texture = self.renderer.texture(512, 512);
                let sampler = self.renderer.sampler(Filter::Nearest, Filter::Nearest);

                let binding = self
                    .sprite_pipeline
                    .binding(&self.renderer, &texture, &sampler);
                loaded_font.binding = Some(binding);
                loaded_font.texture = Some(texture);
            }
            let mut pixels = Vec::with_capacity(data.len() * 4);
            for p in data.iter() {
                pixels.push(1);
                pixels.push(1);
                pixels.push(1);
                pixels.push(*p);
            }
            let pixels = Rgba8::align(&pixels);

            self.renderer.submit(&[Op::Transfer(
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
            self.swap_chain = self.renderer.swap_chain(w, h, PresentMode::NoVsync);
        }

        // let (mx, my) = (self.size().width / 2.0, self.size().height / 2.0);
        // let buffer = shape2d::Batch::singleton(
        //     Shape::circle(Point::new(mx, size.height as f32 - my), 20., 32)
        //         .fill(Fill::Solid(Rgba::new(1., 0., 0., 1.))),
        // )
        // .finish(&self.renderer);

        let output = self.swap_chain.next();
        let mut frame = self.renderer.frame();

        self.renderer.update_pipeline(
            &self.shape_pipeline,
            kit::ortho(output.width, output.height, Default::default()),
            &mut frame,
        );

        self.renderer.update_pipeline(
            &self.sprite_pipeline,
            kit::ortho(output.width, output.height, Default::default()),
            &mut frame,
        );

        {
            let mut pass = frame.pass(PassOp::Clear(Rgba::TRANSPARENT), &output);
            for command in engine_frame.commands.iter() {
                match command {
                    FrameCommand::LoadTexture(texture_handle) => {
                        let mut loaded_texture = texture_handle
                            .handle
                            .write()
                            .expect("Error locking texture to load");
                        if loaded_texture.binding.is_none() {
                            let sampler = self.renderer.sampler(Filter::Nearest, Filter::Nearest);

                            let (gpu_texture, texels) = {
                                let texture = loaded_texture
                                    .texture
                                    .handle
                                    .read()
                                    .expect("Error reading texture");
                                let (w, h) = texture.image.dimensions();
                                let pixels = texture.image.pixels().map(|p| *p).collect::<Vec<_>>();
                                let pixels = Rgba8::align(&pixels);

                                (self.renderer.texture(w as u32, h as u32), pixels.to_owned())
                            };

                            self.renderer
                                .submit(&[Op::Fill(&gpu_texture, texels.as_slice())]);

                            loaded_texture.binding = Some(self.sprite_pipeline.binding(
                                &self.renderer,
                                &gpu_texture,
                                &sampler,
                            ));
                        }
                    }
                    FrameCommand::DrawBatch(batch_handle) => {
                        let batch = batch_handle.read().expect("Error locking batch to render");
                        let loaded_texture = batch
                            .loaded_texture
                            .handle
                            .read()
                            .expect("Error locking texture to render");
                        let texture = loaded_texture
                            .texture
                            .handle
                            .read()
                            .expect("Error reading texture");

                        let mut gpu_batch =
                            sprite2d::Batch::new(texture.image.width(), texture.image.height());
                        for sprite_handle in batch.sprites.iter() {
                            let sprite = sprite_handle
                                .handle
                                .read()
                                .expect("Error locking sprite to render");
                            let source = sprite
                                .source
                                .handle
                                .read()
                                .expect("Error locking source to render");
                            gpu_batch.add(
                                Rect::new(
                                    source.location.x1 as f32,
                                    source.location.y1 as f32,
                                    source.location.x2 as f32,
                                    source.location.y2 as f32,
                                ),
                                sprite.render_at,
                                ZDepth::default(),
                                Rgba::new(1.0, 1.0, 1.0, 0.0),
                                1.0,
                                Repeat::default(),
                            );
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
                    FrameCommand::DrawText { text, loaded_font } => {
                        let text_data = text.handle.read().expect("Error locking text to render");
                        let loaded_font_data = loaded_font
                            .handle
                            .read()
                            .expect("Error locking font to render");
                        if let Some(texture) = loaded_font_data.texture.as_ref() {
                            let font = loaded_font_data
                                .font
                                .handle
                                .read()
                                .expect("Error locking font for rendering");
                            let v_metrics = font
                                .font
                                .v_metrics(rusttype::Scale::uniform(text_data.size));
                            // let batch = sprite2d::Batch::singleton(
                            //     512,
                            //     512,
                            //     Rect::new(0.0, 0.0, 512.0, 512.0),
                            //     Rect::new(0.0, 0.0, 512.0, 512.0),
                            //     ZDepth::default(),
                            //     Rgba::RED,
                            //     1.0,
                            //     Repeat::default(),
                            // );
                            let mut batch = sprite2d::Batch::new(texture.w, texture.h);
                            for (uv_rect, screen_rect) in text_data
                                .positioned_glyphs
                                .as_ref()
                                .expect("Rendering text with no glyphs")
                                .iter()
                                .filter_map(|g| {
                                    loaded_font_data.cache.rect_for(0, g).ok().flatten()
                                })
                            {
                                // This is one section that feels like a kludge. gpu_cache is storing the textures upside down like normal
                                // but rgx is automatically flipping textures. Rgx isn't exactly the best compatibility with this process
                                // because gpu_cache also produces data that is 1 byte per pixel, and we have to expand it when we're updating the texture
                                let source = Rect::new(
                                    uv_rect.min.x * 512.0,
                                    (1.0 - uv_rect.min.y) * 512.0,
                                    uv_rect.max.x * 512.0,
                                    (1.0 - uv_rect.max.y) * 512.0,
                                );

                                // For the destination, we need to invert the y coordinate because without that step, the baseline will be at the top
                                // of the text, not at the bottom.
                                let dest = Rect::new(
                                    text_data.location.x + screen_rect.min.x as f32,
                                    text_data.location.y
                                        + (v_metrics.ascent - screen_rect.min.y as f32),
                                    text_data.location.x + screen_rect.max.x as f32,
                                    text_data.location.y
                                        + (v_metrics.ascent - screen_rect.max.y as f32),
                                );
                                batch.add(
                                    source,
                                    dest,
                                    ZDepth::default(),
                                    Rgba::RED,
                                    1.0,
                                    Repeat::default(),
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

            // pass.set_pipeline(&self.shape_pipeline);
            // pass.draw_buffer(&buffer);
        }

        self.renderer.present(frame);

        Ok(())
    }
}
