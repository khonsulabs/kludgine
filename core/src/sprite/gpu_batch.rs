use easygpu::prelude::*;
use figures::{Rectlike, Round};

use crate::{
    math::{ExtentsRect, Pixels, Point, Rect, Size, Unknown},
    sprite::{pipeline::Vertex, RenderedSprite, SpriteRotation, SpriteSourceLocation},
};

pub struct GpuBatch {
    pub size: Size<u32, ScreenSpace>,
    pub clip: Option<ExtentsRect<u32, Pixels>>,

    items: Vec<Vertex>,
    indicies: Vec<u16>,
}

impl GpuBatch {
    pub fn new(size: Size<u32, ScreenSpace>, clip: Option<ExtentsRect<u32, Pixels>>) -> Self {
        Self {
            size,
            clip,
            items: Vec::default(),
            indicies: Vec::default(),
        }
    }

    pub fn add_sprite(&mut self, sprite: RenderedSprite) {
        let sprite = sprite.data;
        let white_transparent = Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 0,
        };

        match &sprite.source.location {
            SpriteSourceLocation::Rect(location) => self.add_box(
                location.as_extents(),
                sprite.render_at,
                sprite.rotation,
                white_transparent,
            ),
            SpriteSourceLocation::Joined(locations) => {
                let source_bounds = sprite.source.location.bounds();
                let scale_x = sprite.render_at.width().get() / source_bounds.size.width as f32;
                let scale_y = sprite.render_at.height().get() / source_bounds.size.height as f32;
                for location in locations {
                    let x =
                        scale_x.mul_add(location.destination.x as f32, sprite.render_at.origin.x);
                    let y =
                        scale_y.mul_add(location.destination.y as f32, sprite.render_at.origin.y);
                    let width = location.source.width().get() as f32 * scale_x;
                    let height = location.source.height().get() as f32 * scale_y;
                    let destination = Rect::new(Point::new(x, y), Size::new(width, height));
                    self.add_box(
                        location.source.as_extents(),
                        destination.as_extents(),
                        sprite.rotation,
                        white_transparent,
                    );
                }
            }
        }
    }

    pub fn vertex(
        &self,
        src: Point<f32, Unknown>,
        dest: Point<f32, Pixels>,
        color: Rgba8,
    ) -> Vertex {
        Vertex {
            position: [dest.x, dest.y, 0.],
            uv: [
                src.x / self.size.width as f32,
                src.y / self.size.height as f32,
            ],
            color,
        }
    }

    pub fn add_box(
        &mut self,
        src: ExtentsRect<u32, Unknown>,
        mut dest: ExtentsRect<f32, Pixels>,
        rotation: SpriteRotation<Pixels>,
        color: Rgba8,
    ) {
        let mut src = src.cast::<f32>();
        if let Some(clip) = &self.clip {
            // Convert to i32 because the destination could have negative coordinates.
            let clip_signed = clip.cast::<i32>();
            let dest_rounded = dest.round().cast::<i32>();

            if !(clip_signed.origin.x as i32 <= dest_rounded.origin.x
                && clip_signed.origin.y as i32 <= dest_rounded.origin.y
                && clip_signed.extent.x as i32 >= dest_rounded.extent.x
                && clip_signed.extent.y as i32 >= dest_rounded.extent.y)
            {
                if let Some(clipped_destination) = dest.intersection(&clip.cast::<f32>()) {
                    if rotation.angle.is_some() {
                        // To properly apply clipping on a rotated quad requires tessellating the
                        // remaining polygon, and the easygpu-lyon layer doesn't support uv
                        // coordinate extrapolation at this moment. We could use lyon directly to
                        // generate these vertexes.
                        eprintln!(
                            "Kludgine Error: Need to implement partial occlusion for sprites. Not \
                             clipping."
                        );
                    } else {
                        // Adjust the src box based on how much was clipped
                        let source_size = src.size();
                        let dest_size = dest.size();
                        let x_scale = source_size.width / dest_size.width;
                        let y_scale = source_size.height / dest_size.height;
                        src = ExtentsRect::new(
                            Point::new(
                                x_scale.mul_add(
                                    clipped_destination.origin.x - dest.origin.x,
                                    src.origin.x,
                                ),
                                y_scale.mul_add(
                                    clipped_destination.origin.y - dest.origin.y,
                                    src.origin.y,
                                ),
                            ),
                            Point::new(
                                src.extent.x
                                    - (dest.extent.x - clipped_destination.extent.x) * x_scale,
                                src.extent.y
                                    - (dest.extent.y - clipped_destination.extent.y) * y_scale,
                            ),
                        );
                        dest = clipped_destination;
                    }
                } else {
                    // Full clipping, just skip the drawing entirely
                    return;
                }
            }
        }

        let origin = rotation.location.unwrap_or_else(|| dest.center());
        let top_left = self
            .vertex(src.origin, dest.origin, color)
            .rotate_by(rotation.angle, origin);
        let top_right = self
            .vertex(
                Point::from_figures(src.extent.x(), src.origin.y()),
                Point::from_figures(dest.extent.x(), dest.origin.y()),
                color,
            )
            .rotate_by(rotation.angle, origin);
        let bottom_left = self
            .vertex(
                Point::from_figures(src.origin.x(), src.extent.y()),
                Point::from_figures(dest.origin.x(), dest.extent.y()),
                color,
            )
            .rotate_by(rotation.angle, origin);
        let bottom_right = self
            .vertex(
                Point::from_figures(src.extent.x(), src.extent.y()),
                Point::from_figures(dest.extent.x(), dest.extent.y()),
                color,
            )
            .rotate_by(rotation.angle, origin);

        self.add_quad(top_left, top_right, bottom_left, bottom_right);
    }

    pub fn add_quad(
        &mut self,
        top_left: Vertex,
        top_right: Vertex,
        bottom_left: Vertex,
        bottom_right: Vertex,
    ) {
        let top_left_index = self.items.len() as u16;
        self.items.push(top_left);
        let top_right_index = self.items.len() as u16;
        self.items.push(top_right);
        let bottom_left_index = self.items.len() as u16;
        self.items.push(bottom_left);
        let bottom_right_index = self.items.len() as u16;
        self.items.push(bottom_right);

        self.indicies.push(top_left_index);
        self.indicies.push(top_right_index);
        self.indicies.push(bottom_left_index);

        self.indicies.push(top_right_index);
        self.indicies.push(bottom_right_index);
        self.indicies.push(bottom_left_index);
    }

    // pub fn add_triangle(&mut self, a: Vertex, b: Vertex, c: Vertex) {
    //     self.indicies.push(self.indicies.len() as u16);
    //     self.items.push(a);
    //     self.indicies.push(self.indicies.len() as u16);
    //     self.items.push(b);
    //     self.indicies.push(self.indicies.len() as u16);
    //     self.items.push(c);
    // }

    pub(crate) fn finish(&self, renderer: &Renderer) -> BatchBuffers {
        let vertices = renderer.device.create_buffer(&self.items);
        let indices = renderer.device.create_index(&self.indicies);
        BatchBuffers {
            vertices,
            indices,
            index_count: self.indicies.len() as u32,
        }
    }
}

pub struct BatchBuffers {
    pub vertices: VertexBuffer,
    pub indices: IndexBuffer,
    pub index_count: u32,
}

impl Draw for BatchBuffers {
    fn draw<'a, 'b>(&'a self, binding: &'a BindingGroup, pass: &'b mut wgpu::RenderPass<'a>) {
        if self.index_count > 0 {
            pass.set_binding(binding, &[]);
            pass.set_easy_vertex_buffer(&self.vertices);
            pass.set_easy_index_buffer(&self.indices);
            pass.draw_indexed(0..self.index_count as u32, 0, 0..1);
        }
    }
}
