use crate::{
    math::{Point, PointExt, Raw, Rect, Size, Unknown},
    sprite::{pipeline::Vertex, RenderedSprite, SpriteRotation},
};
use euclid::Box2D;
use rgx::{
    color::Rgba8,
    core,
    math::{Vector2, Vector3},
};

use super::SpriteSourceLocation;

pub(crate) struct GpuBatch {
    pub width: u32,
    pub height: u32,

    items: Vec<Vertex>,
    indicies: Vec<u16>,
}

impl GpuBatch {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            items: Default::default(),
            indicies: Default::default(),
        }
    }

    pub async fn add_sprite(&mut self, sprite: RenderedSprite) {
        let sprite = sprite.handle.read().await;
        let source = sprite.source.handle.read().await;
        let white_transparent = Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 0,
        };

        match &source.location {
            SpriteSourceLocation::Rect(location) => self.add_box(
                location.to_box2d(),
                sprite.render_at.to_box2d().round(),
                sprite.rotation,
                white_transparent,
            ),
            SpriteSourceLocation::Joined(locations) => {
                let source_bounds = source.location.bounds();
                let scale_x = sprite.render_at.size.width as f32 / source_bounds.size.width as f32;
                let scale_y =
                    sprite.render_at.size.height as f32 / source_bounds.size.height as f32;
                for location in locations {
                    let x = sprite.render_at.origin.x + location.destination.x as f32 * scale_x;
                    let y = sprite.render_at.origin.y + location.destination.y as f32 * scale_y;
                    let width = sprite.render_at.size.width * scale_x;
                    let height = sprite.render_at.size.height * scale_y;
                    let destination = Rect::new(Point::new(x, y), Size::new(width, height));
                    self.add_box(
                        location.source.to_box2d(),
                        destination.to_box2d().round(),
                        sprite.rotation,
                        white_transparent,
                    );
                }
            }
        }
    }

    pub fn vertex(&self, src: Point<u32, Unknown>, dest: Point<f32, Raw>, color: Rgba8) -> Vertex {
        Vertex {
            position: Vector3::new(dest.x, dest.y, 0.),
            uv: Vector2::new(
                src.x as f32 / self.width as f32,
                src.y as f32 / self.height as f32,
            ),
            color,
        }
    }

    pub fn add_box(
        &mut self,
        src: Box2D<u32, Unknown>,
        dest: Box2D<f32, Raw>,
        rotation: SpriteRotation<Raw>,
        color: Rgba8,
    ) {
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

    pub fn finish(&self, renderer: &core::Renderer) -> BatchBuffers {
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
    vertices: core::VertexBuffer,
    indices: core::IndexBuffer,
    index_count: u32,
}

impl rgx::core::Draw for BatchBuffers {
    fn draw(&self, binding: &rgx::core::BindingGroup, pass: &mut rgx::core::Pass) {
        pass.set_binding(binding, &[]);
        pass.set_vertex_buffer(&self.vertices);
        pass.set_index_buffer(&self.indices);
        pass.draw_indexed(0..self.index_count as u32, 0..1);
    }
}
