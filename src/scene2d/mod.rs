mod perspective;
mod screen;
pub use perspective::PerspectiveScene;
pub use screen::ScreenScene;

use crate::internal_prelude::*;
use crate::materials::Material;
use cgmath::Rad;
use cgmath::{Matrix4, Vector2, Vector3};
use generational_arena::Arena;
use gl::types::*;
use lyon::tessellation::{
    basic_shapes::fill_rectangle, BasicGeometryBuilder, Count, FillAttributes, FillGeometryBuilder,
    FillOptions, GeometryBuilder, GeometryBuilderError, StrokeAttributes, StrokeGeometryBuilder,
    VertexId,
};
use std::mem;
use std::os::raw::c_void;
use std::{
    cmp::Ordering,
    collections::HashMap,
    ptr,
    sync::{Arc, Mutex},
};

#[derive(Educe)]
#[educe(Default)]
pub(crate) struct PerspectiveSettings {
    #[educe(Default(expression = "Deg(90.0)"))]
    pub(crate) fov: Deg<f32>,
    #[educe(Default = 0.01)]
    pub(crate) znear: f32,
    #[educe(Default = 1000.0)]
    pub(crate) zfar: f32,
}

#[derive(Educe)]
#[educe(Default)]
pub(crate) struct ScreenSettings {
    #[educe(Default = 1.0)]
    pub(crate) scale_factor: f32,
}

pub struct Scene2d {
    pub(crate) arena: Arena<Arc<Mutex<MeshStorage>>>,
    pub(crate) placements: HashMap<generational_arena::Index, Placement2d>,
    pub(crate) size: Size2d,
    pub(crate) screen_settings: ScreenSettings,
    pub(crate) perspective_settings: PerspectiveSettings,
}

pub struct Scene2dNode {}

impl Scene2d {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            size: Size2d::default(),
            placements: HashMap::new(),
            screen_settings: ScreenSettings::default(),
            perspective_settings: PerspectiveSettings::default(),
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
                self.size.width / self.screen_settings.scale_factor,
                self.size.height / self.screen_settings.scale_factor,
                0.0,
                1.0,
                -1.0,
            ),
            Placement2dLocation::Z(_) => cgmath::perspective(
                self.perspective_settings.fov,
                self.size.width / self.size.height,
                self.perspective_settings.znear,
                self.perspective_settings.zfar,
            ),
        }
    }

    pub fn create_mesh<M: Into<Material>>(&mut self, shape: Shape, material: M) -> Mesh2d {
        let material = material.into();
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

impl GeometryBuilder for Shape {
    fn begin_geometry(&mut self) {
        let mut storage = self.storage.lock().expect("Error locking ShapeStorage");
        storage.vertices.clear();
    }

    fn end_geometry(&mut self) -> Count {
        let storage = self.storage.lock().expect("Error locking ShapeStorage");
        Count {
            vertices: (storage.vertices.len()) as u32,
            indices: storage.triangle_verticies.len() as u32,
        }
    }

    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        let mut storage = self.storage.lock().expect("Error locking ShapeStorage");
        storage.triangle_verticies.push(a.0);
        storage.triangle_verticies.push(b.0);
        storage.triangle_verticies.push(c.0);
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
        storage
            .vertices
            .push(Vector3::new(position.x, position.y, 0.0));
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
        storage
            .vertices
            .push(Vector3::new(position.x, position.y, 0.0));
        Ok(VertexId(storage.vertices.len() as u32 - 1))
    }
}

impl BasicGeometryBuilder for Shape {
    fn add_vertex(&mut self, position: Point2d) -> Result<VertexId, GeometryBuilderError> {
        let mut storage = self.storage.lock().expect("Error locking ShapeStorage");
        if storage.vertices.len() as u32 >= std::u32::MAX {
            return Err(GeometryBuilderError::TooManyVertices);
        }
        storage
            .vertices
            .push(Vector3::new(position.x, position.y, 0.0));
        Ok(VertexId(storage.vertices.len() as u32 - 1))
    }
}

#[derive(Clone)]
pub struct Shape {
    pub(crate) storage: Arc<Mutex<ShapeStorage>>,
}

#[derive(Clone)]
pub(crate) struct CompiledShape {
    pub vao: u32,
    pub ebo: u32,
    pub vbo: u32,
    pub count: i32,
}

impl Drop for CompiledShape {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vbo);
            self.vbo = 0;
            gl::DeleteBuffers(1, &self.ebo);
            self.ebo = 0;
            gl::DeleteVertexArrays(1, &self.vao);
            self.vao = 0;
        }
    }
}

impl CompiledShape {
    pub fn activate(&self) {
        unsafe { gl::BindVertexArray(self.vao) };
    }
}

#[derive(Default)]
pub(crate) struct ShapeStorage {
    pub vertices: Vec<Vector3<f32>>,
    pub texture_coordinates: Vec<Vector2<f32>>,
    pub triangle_verticies: Vec<u32>,
    pub(crate) compiled: Option<Arc<CompiledShape>>,
}

impl Shape {
    pub fn rect(r: &Rect) -> Self {
        let mut shape = Self::default();
        fill_rectangle(r, &FillOptions::default(), &mut shape).expect("Error generating rectangle");
        shape
    }

    pub(crate) fn compile(&self) -> Arc<CompiledShape> {
        let mut shape = self.storage.lock().expect("Error locking shape");
        if let None = &shape.compiled {
            let (vao, ebo, vbo) = unsafe {
                let (mut vbo, mut vao, mut ebo) = (0, 0, 0);
                gl::GenVertexArrays(1, &mut vao);
                gl::GenBuffers(1, &mut vbo);
                gl::GenBuffers(1, &mut ebo);
                // bind the Vertex Array Object first, then bind and set vertex buffer(s), and then configure vertex attributes(s).
                gl::BindVertexArray(vao);

                gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (shape.vertices.len() * mem::size_of::<f32>() * 3) as GLsizeiptr,
                    shape.vertices.as_ptr() as *const c_void,
                    gl::STATIC_DRAW,
                );

                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
                gl::BufferData(
                    gl::ELEMENT_ARRAY_BUFFER,
                    (shape.triangle_verticies.len() * mem::size_of::<u32>()) as GLsizeiptr,
                    shape.triangle_verticies.as_ptr() as *const c_void,
                    gl::STATIC_DRAW,
                );
                gl::VertexAttribPointer(
                    0,
                    3,
                    gl::FLOAT,
                    gl::FALSE,
                    3 * mem::size_of::<f32>() as GLsizei,
                    ptr::null(),
                );
                gl::EnableVertexAttribArray(0);
                assert!(gl::GetError() == 0);

                (vao, ebo, vbo)
            };
            shape.compiled = Some(Arc::new(CompiledShape {
                vao,
                ebo,
                vbo,
                count: shape.triangle_verticies.len() as i32,
            }))
        }

        shape.compiled.as_ref().unwrap().clone()
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
