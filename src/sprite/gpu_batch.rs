use crate::{
    math::{Point, PointExt, Raw, Unknown},
    sprite::{pipeline::Vertex, RenderedSprite},
};
use euclid::Box2D;
use rgx::{
    color::Rgba8,
    core,
    math::{Vector2, Vector3},
};

pub(crate) struct GpuBatch {
    pub width: u32,
    pub height: u32,

    items: Vec<Vertex>,
}

impl GpuBatch {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            items: Default::default(),
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

        self.add_box(
            source.location.to_box2d(),
            sprite.render_at.to_box2d(),
            white_transparent,
        )
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

    pub fn add_box(&mut self, src: Box2D<u32, Unknown>, dest: Box2D<f32, Raw>, color: Rgba8) {
        let top_left = self.vertex(src.min, dest.min, color);
        let top_right = self.vertex(
            Point::from_lengths(src.max.x(), src.min.y()),
            Point::from_lengths(dest.max.x(), dest.min.y()),
            color,
        );
        let bottom_left = self.vertex(
            Point::from_lengths(src.min.x(), src.max.y()),
            Point::from_lengths(dest.min.x(), dest.max.y()),
            color,
        );
        let bottom_right = self.vertex(
            Point::from_lengths(src.max.x(), src.max.y()),
            Point::from_lengths(dest.max.x(), dest.max.y()),
            color,
        );

        self.add_triangle(top_left, top_right, bottom_left);
        self.add_triangle(top_right, bottom_right, bottom_left);
    }

    pub fn add_triangle(&mut self, a: Vertex, b: Vertex, c: Vertex) {
        self.items.push(a);
        self.items.push(b);
        self.items.push(c);
    }

    pub fn finish(&self, renderer: &core::Renderer) -> core::VertexBuffer {
        renderer.device.create_buffer(&self.items)
    }
}
