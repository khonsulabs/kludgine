use crate::{
    math::{Box2D, Point, PointExt, Raw, Rect, Size, Unknown},
    sprite::{pipeline::Vertex, RenderedSprite, SpriteRotation, SpriteSourceLocation},
};
use easygpu::prelude::*;
use euclid::{Vector2D, Vector3D};

pub(crate) struct GpuBatch {
    pub size: Size<u32, ScreenSpace>,
    pub clip: Option<Box2D<u32, Raw>>,

    items: Vec<Vertex>,
    indicies: Vec<u16>,
}

impl GpuBatch {
    pub fn new(size: Size<u32, ScreenSpace>, clip: Option<Box2D<u32, Raw>>) -> Self {
        Self {
            size,
            clip,
            items: Default::default(),
            indicies: Default::default(),
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
                location.to_box2d(),
                sprite.render_at,
                sprite.rotation,
                white_transparent,
            ),
            SpriteSourceLocation::Joined(locations) => {
                let source_bounds = sprite.source.location.bounds();
                let scale_x = sprite.render_at.width() as f32 / source_bounds.size.width as f32;
                let scale_y = sprite.render_at.height() as f32 / source_bounds.size.height as f32;
                for location in locations {
                    let x = sprite.render_at.min.x + location.destination.x as f32 * scale_x;
                    let y = sprite.render_at.min.y + location.destination.y as f32 * scale_y;
                    let width = location.source.width() as f32 * scale_x;
                    let height = location.source.height() as f32 * scale_y;
                    let destination = Rect::new(Point::new(x, y), Size::new(width, height));
                    self.add_box(
                        location.source.to_box2d(),
                        destination.to_box2d(),
                        sprite.rotation,
                        white_transparent,
                    );
                }
            }
        }
    }

    pub fn vertex(&self, src: Point<f32, Unknown>, dest: Point<f32, Raw>, color: Rgba8) -> Vertex {
        Vertex {
            position: Vector3D::new(dest.x, dest.y, 0.),
            uv: Vector2D::new(
                src.x / self.size.width as f32,
                src.y / self.size.height as f32,
            ),
            color,
        }
    }

    pub fn add_box(
        &mut self,
        src: Box2D<u32, Unknown>,
        mut dest: Box2D<f32, Raw>,
        rotation: SpriteRotation<Raw>,
        color: Rgba8,
    ) {
        let mut src = src.to_f32();
        if let Some(clip) = &self.clip {
            // Convert to i32 because the destination could have negative coordinates.
            let clip_signed = clip.to_i32();
            let dest_rounded = dest.round().to_i32();

            if !(clip_signed.min.x as i32 <= dest_rounded.min.x
                && clip_signed.min.y as i32 <= dest_rounded.min.y
                && clip_signed.max.x as i32 >= dest_rounded.max.x
                && clip_signed.max.y as i32 >= dest_rounded.max.y)
            {
                if let Some(clipped_destination) = dest.intersection(&clip.to_f32()) {
                    if rotation.angle.is_some() {
                        // To properly apply clipping on a rotated quad requires tessellating the remaining polygon, and
                        // the easygpu-lyon layer doesn't support uv coordinate extrapolation at this moment. We could use
                        // lyon directly to generate these vertexes.
                        eprintln!("Kludgine Error: Need to implement partial occlusion for sprites. Not clipping.");
                    } else {
                        // Adjust the src box based on how much was clipped
                        let source_size = src.size();
                        let dest_size = dest.size();
                        let x_scale = source_size.width / dest_size.width;
                        let y_scale = source_size.height / dest_size.height;
                        src = Box2D::new(
                            Point::new(
                                src.min.x + (clipped_destination.min.x - dest.min.x) * x_scale,
                                src.min.y + (clipped_destination.min.y - dest.min.y) * y_scale,
                            ),
                            Point::new(
                                src.max.x - (dest.max.x - clipped_destination.max.x) * x_scale,
                                src.max.y - (dest.max.y - clipped_destination.max.y) * y_scale,
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

        let origin = rotation.screen_location.unwrap_or_else(|| dest.center());
        let top_left = self
            .vertex(src.min, dest.min, color)
            .rotate_by(rotation.angle, origin);
        let top_right = self
            .vertex(
                Point::from_lengths(src.max.x(), src.min.y()),
                Point::from_lengths(dest.max.x(), dest.min.y()),
                color,
            )
            .rotate_by(rotation.angle, origin);
        let bottom_left = self
            .vertex(
                Point::from_lengths(src.min.x(), src.max.y()),
                Point::from_lengths(dest.min.x(), dest.max.y()),
                color,
            )
            .rotate_by(rotation.angle, origin);
        let bottom_right = self
            .vertex(
                Point::from_lengths(src.max.x(), src.max.y()),
                Point::from_lengths(dest.max.x(), dest.max.y()),
                color,
            )
            .rotate_by(rotation.angle, origin);

        self.add_quad(top_left, top_right, bottom_left, bottom_right);
    }

    pub fn add_quad(&mut self, tl: Vertex, tr: Vertex, bl: Vertex, br: Vertex) {
        let tl_index = self.items.len() as u16;
        self.items.push(tl);
        let tr_index = self.items.len() as u16;
        self.items.push(tr);
        let bl_index = self.items.len() as u16;
        self.items.push(bl);
        let br_index = self.items.len() as u16;
        self.items.push(br);

        self.indicies.push(tl_index);
        self.indicies.push(tr_index);
        self.indicies.push(bl_index);

        self.indicies.push(tr_index);
        self.indicies.push(br_index);
        self.indicies.push(bl_index);
    }

    // pub fn add_triangle(&mut self, a: Vertex, b: Vertex, c: Vertex) {
    //     self.indicies.push(self.indicies.len() as u16);
    //     self.items.push(a);
    //     self.indicies.push(self.indicies.len() as u16);
    //     self.items.push(b);
    //     self.indicies.push(self.indicies.len() as u16);
    //     self.items.push(c);
    // }

    pub fn finish(&self, renderer: &Renderer) -> BatchBuffers {
        let vertices = renderer.device.create_buffer(&self.items);
        let indices = renderer.device.create_index(&self.indicies);
        BatchBuffers {
            vertices,
            indices,
            index_count: self.indicies.len() as u32,
        }
    }
}

pub(crate) struct BatchBuffers {
    pub vertices: VertexBuffer,
    pub indices: IndexBuffer,
    pub index_count: u32,
}

impl Draw for BatchBuffers {
    fn draw<'a, 'b>(&'a self, binding: &'a BindingGroup, pass: &'b mut wgpu::RenderPass<'a>) {
        pass.set_binding(binding, &[]);
        pass.set_easy_vertex_buffer(&self.vertices);
        pass.set_easy_index_buffer(&self.indices);
        pass.draw_indexed(0..self.index_count as u32, 0, 0..1);
    }
}
