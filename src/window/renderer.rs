use crate::{
    runtime::{Runtime, FRAME_DURATION},
    scene::{Frame, FrameCommand},
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
                    println!("Closing window thread");
                    return;
                }

                async_std::task::sleep(remaining).await;
            }
            self.render().expect("Error rendering window");
            limiter.advance_frame();
        }
    }

    pub fn render(&mut self) -> KludgineResult<()> {
        let engine_frame = self.frame.read().expect("Error reading frame");
        let (w, h) = {
            (
                engine_frame.size.width as u32,
                engine_frame.size.height as u32,
            )
        };
        if w == 0 || h == 0 {
            return Ok(());
        }

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
                            .write()
                            .expect("Error locking texture to load");
                        if loaded_texture.binding.is_none() {
                            let sampler = self.renderer.sampler(Filter::Nearest, Filter::Nearest);

                            let (gpu_texture, texels) = {
                                let texture = loaded_texture
                                    .texture
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
                            .read()
                            .expect("Error locking texture to render");
                        let texture = loaded_texture
                            .texture
                            .read()
                            .expect("Error reading texture");

                        let mut gpu_batch =
                            sprite2d::Batch::new(texture.image.width(), texture.image.height());
                        for sprite_handle in batch.sprites.iter() {
                            let sprite = sprite_handle
                                .read()
                                .expect("Error locking sprite to render");
                            let source = sprite
                                .source
                                .read()
                                .expect("Error locking source to render");
                            gpu_batch.add(
                                source.location,
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
                }
            }

            // pass.set_pipeline(&self.shape_pipeline);
            // pass.draw_buffer(&buffer);
        }

        self.renderer.present(frame);

        Ok(())
    }
}
