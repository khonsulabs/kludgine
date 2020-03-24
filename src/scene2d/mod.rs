mod perspective;
mod screen;
pub use perspective::PerspectiveScene;
pub use screen::ScreenScene;

use crate::internal_prelude::*;
use crate::materials::Material;
use cgmath::Matrix4;
use cgmath::Rad;
use generational_arena::Arena;
use lyon::tessellation::{
    basic_shapes::fill_rectangle, BasicGeometryBuilder, Count, FillAttributes, FillGeometryBuilder,
    FillOptions, GeometryBuilder, GeometryBuilderError, StrokeAttributes, StrokeGeometryBuilder,
    VertexId,
};
use std::{
    cmp::Ordering,
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub struct Scene2d {
    pub(crate) arena: Arena<Arc<Mutex<MeshStorage>>>,
    pub(crate) placements: HashMap<generational_arena::Index, Placement2d>,
    pub(crate) size: Size2d,
    pub(crate) scale_factor: f32,
}

pub struct Scene2dNode {}

impl Scene2d {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            size: Size2d::default(),
            placements: HashMap::new(),
            scale_factor: 1.0,
        }
    }

    pub fn screen<'a>(&'a mut self) -> ScreenScene<'a> {
        ScreenScene { scene: self }
    }

    pub fn perspective<'a>(&'a mut self) -> PerspectiveScene<'a> {
        PerspectiveScene { scene: self }
    }

    pub fn size(&self) -> Size2d {
        self.size
    }

    pub(crate) fn projection(&self, location: &Placement2dLocation) -> Matrix4<f32> {
        match location {
            Placement2dLocation::Screen => cgmath::ortho(
                0.0,
                self.size.width / self.scale_factor,
                self.size.height / self.scale_factor,
                0.0,
                1.0,
                -1.0,
            ),
            Placement2dLocation::Z(_) => {
                cgmath::perspective(Deg(110.0), self.size.width / self.size.height, 0.01, 10.0)
            }
        }
    }

    pub fn create_mesh(&mut self, shape: Shape, material: Material) -> Mesh2d {
        let storage = Arc::new(Mutex::new(MeshStorage {
            shape,
            material,
            angle: Rad(0.0),
            scale: 1.0,
            position: Point2d::new(0.0, 0.0),
        }));
        let id = self.arena.insert(storage.clone());
        Mesh2d { id, storage }
    }

    pub fn create_mesh_clone(&mut self, copy: &Mesh2d) -> Mesh2d {
        let copy_storage = copy.storage.lock().expect("Error locking copy storage");
        let storage = Arc::new(Mutex::new(MeshStorage {
            shape: copy_storage.shape.clone(),
            material: copy_storage.material.clone(),
            angle: copy_storage.angle,
            scale: copy_storage.scale,
            position: copy_storage.position,
        }));
        let id = self.arena.insert(storage.clone());
        Mesh2d { id, storage }
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
        position: Point2d,
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
        position: Point2d,
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

impl BasicGeometryBuilder for Shape {
    fn add_vertex(&mut self, position: Point2d) -> Result<VertexId, GeometryBuilderError> {
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
    pub(crate) storage: Arc<Mutex<ShapeStorage>>,
}

#[derive(Default)]
pub(crate) struct ShapeStorage {
    pub vertices: Vec<Point2d>,
    pub texture_coordinates: Vec<Point2d>,
    pub triangles: Vec<(VertexId, VertexId, VertexId)>,
}

impl Shape {
    pub fn rect(r: &Rect) -> Self {
        let mut shape = Self::default();
        fill_rectangle(r, &FillOptions::default(), &mut shape).expect("Error generating rectangle");
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
pub struct Mesh2d {
    pub id: generational_arena::Index,
    pub(crate) storage: Arc<Mutex<MeshStorage>>,
}

pub(crate) struct MeshStorage {
    pub shape: Shape,
    pub material: Material,
    pub position: Point2d,
    pub scale: f32,
    pub angle: Rad<f32>,
}

pub mod prelude {
    pub use super::{Mesh2d, Scene2d, ScreenScene, Shape};
}

pub(crate) struct Placement2d {
    pub mesh: Mesh2d,
    pub relative_to: Option<generational_arena::Index>,
    pub position: Point2d,
    pub angle: Rad<f32>,
    pub scale: f32,
    pub location: Placement2dLocation,
}

#[derive(PartialEq)]
pub(crate) enum Placement2dLocation {
    Screen,
    Z(f32),
}

impl Placement2d {
    pub fn z(&self) -> f32 {
        match self.location {
            Placement2dLocation::Screen => 0.0,
            Placement2dLocation::Z(z) => z,
        }
    }
}

impl std::cmp::PartialOrd for Placement2dLocation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            Placement2dLocation::Screen => {
                match other {
                    Placement2dLocation::Screen => Some(Ordering::Equal),
                    Placement2dLocation::Z(_) => Some(Ordering::Greater), // Screen is higher than Z
                }
            }
            Placement2dLocation::Z(z) => {
                match other {
                    Placement2dLocation::Screen => Some(Ordering::Less), // Z is lower than Screen
                    Placement2dLocation::Z(other_z) => z.partial_cmp(&other_z),
                }
            }
        }
    }
}
