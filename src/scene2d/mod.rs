use euclid::{Point2D, Rect, Size2D, UnknownUnit};
use generational_arena::Arena;
use lyon::tessellation::{
    basic_shapes::stroke_rectangle, BasicVertexConstructor, Count, FillAttributes,
    FillGeometryBuilder, GeometryBuilder, GeometryBuilderError, StrokeAttributes,
    StrokeGeometryBuilder, StrokeOptions, VertexBuffers, VertexId,
};
use nalgebra::{Vector2, Vector4};
use std::sync::{Arc, Mutex};

pub type Point = Point2D<f32, UnknownUnit>;
pub type Rectangle = Rect<f32, UnknownUnit>;
pub type Size = Size2D<f32, UnknownUnit>;

pub struct Scene2D {
    arena: Mutex<Arena<()>>,
}

impl Scene2D {
    pub fn new() -> Self {
        Self {
            arena: Mutex::new(Arena::new()),
        }
    }

    pub fn screen<'a>(&'a self) -> ScreenScene<'a> {
        ScreenScene { scene: self }
    }
}

pub struct ScreenScene<'a> {
    scene: &'a Scene2D,
}

pub struct Mesh {}

impl GeometryBuilder for Shape {
    fn begin_geometry(&mut self) {
        let mut storage = self.storage.lock().expect("Error locking ShapeStorage");
        storage.vertices.clear();
    }

    fn end_geometry(&mut self) -> Count {
        let storage = self.storage.lock().expect("Error locking ShapeStorage");
        Count {
            vertices: (storage.vertices.len()) as u32,
            indices: (storage.triangles.len() * 3) as u32,
        }
    }

    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        let mut storage = self.storage.lock().expect("Error locking ShapeStorage");
        storage.triangles.push((a, b, c));
    }

    fn abort_geometry(&mut self) {
        let mut storage = self.storage.lock().expect("Error locking ShapeStorage");
        storage.vertices.clear();
    }
}

impl FillGeometryBuilder for Shape {
    fn add_fill_vertex(
        &mut self,
        position: Point,
        _: FillAttributes,
    ) -> Result<VertexId, GeometryBuilderError> {
        let mut storage = self.storage.lock().expect("Error locking ShapeStorage");
        if storage.vertices.len() as u32 >= std::u32::MAX {
            return Err(GeometryBuilderError::TooManyVertices);
        }
        storage.vertices.push(position);
        Ok(VertexId(storage.vertices.len() as u32 - 1))
    }
}

impl StrokeGeometryBuilder for Shape {
    fn add_stroke_vertex(
        &mut self,
        position: Point,
        _: StrokeAttributes,
    ) -> Result<VertexId, GeometryBuilderError> {
        let mut storage = self.storage.lock().expect("Error locking ShapeStorage");
        if storage.vertices.len() as u32 >= std::u32::MAX {
            return Err(GeometryBuilderError::TooManyVertices);
        }
        storage.vertices.push(position);
        Ok(VertexId(storage.vertices.len() as u32 - 1))
    }
}
#[derive(Clone)]
pub struct Shape {
    storage: Arc<Mutex<ShapeStorage>>,
}

#[derive(Default)]
struct ShapeStorage {
    vertices: Vec<Point>,
    texture_coordinates: Vec<Point>,
    triangles: Vec<(VertexId, VertexId, VertexId)>,
}

impl Shape {
    pub fn rect(r: &Rectangle) -> Self {
        let mut shape = Self::default();
        stroke_rectangle(r, &StrokeOptions::default(), &mut shape)
            .expect("Error generating rectangle");
        shape
    }
}

impl Default for Shape {
    fn default() -> Self {
        Shape {
            storage: Arc::new(Mutex::new(ShapeStorage::default())),
        }
    }
}
