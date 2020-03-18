use crate::material::Material;
use cgmath::Rad;
use generational_arena::Arena;
use lyon::tessellation::{
    basic_shapes::stroke_rectangle, Count, FillAttributes, FillGeometryBuilder, GeometryBuilder,
    GeometryBuilderError, StrokeAttributes, StrokeGeometryBuilder, StrokeOptions, VertexId,
};
use std::sync::{Arc, Mutex};

pub type Point2D = euclid::Point2D<f32, euclid::UnknownUnit>;
pub type Rect = euclid::Rect<f32, euclid::UnknownUnit>;
pub type Size2D = euclid::Size2D<f32, euclid::UnknownUnit>;

pub struct Scene2D {
    arena: Mutex<Arena<Arc<Mutex<MeshStorage>>>>,
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

impl<'a> ScreenScene<'a> {
    pub fn create_mesh(&self, shape: Shape, material: Material) -> Mesh2D {
        let storage = Arc::new(Mutex::new(MeshStorage {
            shape,
            material,
            angle: Rad(0.0),
            scale: 1.0,
            position: Point2D::new(0.0, 0.0),
        }));
        let mut arena = self.scene.arena.lock().expect("Error locking Arena");
        let id = arena.insert(storage.clone());
        Mesh2D { id, storage }
    }
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
        position: Point2D,
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
        position: Point2D,
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
    vertices: Vec<Point2D>,
    texture_coordinates: Vec<Point2D>,
    triangles: Vec<(VertexId, VertexId, VertexId)>,
}

impl Shape {
    pub fn rect(r: &Rect) -> Self {
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

#[derive(Clone)]
pub struct Mesh2D {
    pub id: generational_arena::Index,
    storage: Arc<Mutex<MeshStorage>>,
}

struct MeshStorage {
    pub shape: Shape,
    pub material: Material,
    pub position: Point2D,
    pub scale: f32,
    pub angle: Rad<f32>,
}

pub mod prelude {
    pub use super::{Mesh2D, Point2D, Rect, Scene2D, ScreenScene, Shape, Size2D};
}
