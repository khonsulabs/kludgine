mod perspective;
mod screen;
pub use perspective::PerspectiveScene;
pub use screen::ScreenScene;

use crate::internal_prelude::*;
use crate::materials::Material;
use cgmath::Rad;
use cgmath::{Matrix4, Vector2, Vector3};
use gl::types::*;
use glutin::event::VirtualKeyCode;
use legion::prelude::*;
use lyon::tessellation::{
    basic_shapes::fill_rectangle, BasicGeometryBuilder, Count, FillAttributes, FillGeometryBuilder,
    FillOptions, GeometryBuilder, GeometryBuilderError, StrokeAttributes, StrokeGeometryBuilder,
    VertexId,
};
use std::mem;
use std::os::raw::c_void;
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    ptr,
    sync::{Arc, RwLock},
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
    pub(crate) camera_position: Point3d,
}

#[derive(Educe)]
#[educe(Default)]
pub(crate) struct ScreenSettings {
    #[educe(Default = 1.0)]
    pub(crate) scale_factor: f32,
}

pub struct Scene2d {
    pub universe: Universe,
    pub world: World,
    pub pressed_keys: HashSet<VirtualKeyCode>,
    pub(crate) children: HashMap<Entity, Placement2d>,
    pub(crate) size: Size2d,
    pub(crate) screen_settings: ScreenSettings,
    pub(crate) perspective_settings: PerspectiveSettings,
    pub(crate) lazy_mesh_cache: HashMap<String, Mesh>,
}

impl Scene2d {
    pub fn new() -> Self {
        let universe = Universe::new();
        let world = universe.create_world();
        Self {
            universe,
            world,
            size: Size2d::default(),
            children: HashMap::new(),
            screen_settings: ScreenSettings::default(),
            perspective_settings: PerspectiveSettings::default(),
            lazy_mesh_cache: HashMap::new(),
            pressed_keys: HashSet::new(),
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

    pub fn get(&self, id: Entity) -> Option<Mesh> {
        match self.world.get_component::<MeshHandle>(id) {
            Some(handle) => Some(Mesh {
                id,
                handle: handle.as_ref().clone(),
            }),
            None => None,
        }
    }

    pub fn cached_mesh<S: Into<String>, F: FnOnce(&mut Scene2d) -> KludgineResult<Mesh>>(
        &mut self,
        name: S,
        initializer: F,
    ) -> KludgineResult<Mesh> {
        let name = name.into();
        match self.lazy_mesh_cache.get(&name) {
            Some(mesh) => Ok(mesh.clone()),
            None => {
                let new_mesh = initializer(self)?;
                self.lazy_mesh_cache.insert(name, new_mesh.clone());
                Ok(new_mesh)
            }
        }
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

    pub fn create_mesh<M: Into<Material>>(&mut self, shape: Shape, material: M) -> Mesh {
        let material = material.into();
        let storage = Arc::new(RwLock::new(MeshStorage {
            shape,
            material,
            angle: Rad(0.0),
            scale: 1.0,
            position: Point2d::new(0.0, 0.0),
            children: HashMap::new(),
        }));
        let handle = MeshHandle { storage };
        let id = self.world.insert((), vec![(handle.clone(),)])[0];
        Mesh { id, handle }
    }

    pub fn create_mesh_clone(&mut self, copy: &Mesh) -> Mesh {
        let copy_storage = copy
            .handle
            .storage
            .read()
            .expect("Error locking copy storage");
        let storage = Arc::new(RwLock::new(MeshStorage {
            shape: copy_storage.shape.clone(),
            material: copy_storage.material.clone(),
            angle: copy_storage.angle,
            scale: copy_storage.scale,
            position: copy_storage.position,
            children: HashMap::new(),
        }));
        let handle = MeshHandle { storage };
        let id = self.world.insert((), vec![(handle.clone(),)])[0];
        Mesh { id, handle }
    }
}

impl GeometryBuilder for Shape {
    fn begin_geometry(&mut self) {
        let mut storage = self.storage.write().expect("Error locking ShapeStorage");
        storage.vertices.clear();
    }

    fn end_geometry(&mut self) -> Count {
        let storage = self.storage.read().expect("Error locking ShapeStorage");
        Count {
            vertices: (storage.vertices.len()) as u32,
            indices: storage.triangle_verticies.len() as u32,
        }
    }

    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        let mut storage = self.storage.write().expect("Error locking ShapeStorage");
        storage.triangle_verticies.push(a.0);
        storage.triangle_verticies.push(b.0);
        storage.triangle_verticies.push(c.0);
    }

    fn abort_geometry(&mut self) {
        let mut storage = self.storage.write().expect("Error locking ShapeStorage");
        storage.vertices.clear();
    }
}

impl FillGeometryBuilder for Shape {
    fn add_fill_vertex(
        &mut self,
        position: Point2d,
        _: FillAttributes,
    ) -> Result<VertexId, GeometryBuilderError> {
        let mut storage = self.storage.write().expect("Error locking ShapeStorage");
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
        let mut storage = self.storage.write().expect("Error locking ShapeStorage");
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
        let mut storage = self.storage.write().expect("Error locking ShapeStorage");
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
    pub(crate) storage: Arc<RwLock<ShapeStorage>>,
}

#[derive(Clone)]
pub(crate) struct CompiledShape {
    pub vertex_array_object: u32,
    pub entity_buffer_object: u32,
    pub vertex_buffer_object: u32,
    pub vertex_buffer_texture_coords_object: Option<u32>,
    pub count: i32,
}

impl Drop for CompiledShape {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vertex_buffer_object);
            self.vertex_buffer_object = 0;
            gl::DeleteBuffers(1, &self.entity_buffer_object);
            self.entity_buffer_object = 0;
            gl::DeleteVertexArrays(1, &self.vertex_array_object);
            self.vertex_array_object = 0;
            if let Some(vertex_buffer_texture_coords_object) =
                self.vertex_buffer_texture_coords_object
            {
                gl::DeleteVertexArrays(1, &vertex_buffer_texture_coords_object);
                self.vertex_buffer_texture_coords_object = None;
            }
        }
    }
}

impl CompiledShape {
    pub fn activate(&self) {
        unsafe {
            gl::BindVertexArray(self.vertex_array_object);
        }
    }
}

#[derive(Default)]
pub(crate) struct ShapeStorage {
    pub vertices: Vec<Vector3<f32>>,
    pub texture_coordinates: Option<Vec<Vector2<f32>>>,
    pub triangle_verticies: Vec<u32>,
    pub(crate) compiled: Option<Arc<CompiledShape>>,
}

impl Shape {
    pub fn rect(r: &Rect) -> Self {
        let mut shape = Self::default();
        fill_rectangle(r, &FillOptions::default(), &mut shape).expect("Error generating rectangle");
        shape
    }

    pub fn set_texture_coordinates(&self, texture_coordinates: Vec<Point2d>) {
        let mut shape = self.storage.write().expect("Error locking shape for write");
        shape.texture_coordinates = Some(
            texture_coordinates
                .into_iter()
                .map(|c| Vector2::new(c.x, c.y))
                .collect(),
        );
    }

    pub(crate) fn compile(&self) -> KludgineResult<Arc<CompiledShape>> {
        let mut shape = self.storage.write().expect("Error locking shape");
        if let None = &shape.compiled {
            let (
                vertex_array_object,
                entity_buffer_object,
                vertex_buffer_object,
                vertex_buffer_texture_coords_object,
            ) = unsafe {
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
                gl::VertexAttribPointer(
                    0,
                    3,
                    gl::FLOAT,
                    gl::FALSE,
                    (3 * mem::size_of::<f32>()) as GLsizei,
                    ptr::null(),
                );
                gl::EnableVertexAttribArray(0);

                let vbotx = if let Some(texture_coords) = &shape.texture_coordinates {
                    let mut vbotx = 0;
                    gl::GenBuffers(1, &mut vbotx);
                    gl::BindBuffer(gl::ARRAY_BUFFER, vbotx);
                    gl::BufferData(
                        gl::ARRAY_BUFFER,
                        (texture_coords.len() * mem::size_of::<f32>() * 2) as GLsizeiptr,
                        texture_coords.as_ptr() as *const c_void,
                        gl::STATIC_DRAW,
                    );
                    gl::VertexAttribPointer(
                        1,
                        2,
                        gl::FLOAT,
                        gl::FALSE,
                        (2 * mem::size_of::<f32>()) as GLsizei,
                        ptr::null(),
                    );
                    gl::EnableVertexAttribArray(1);
                    Some(vbotx)
                } else {
                    None
                };

                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
                gl::BufferData(
                    gl::ELEMENT_ARRAY_BUFFER,
                    (shape.triangle_verticies.len() * mem::size_of::<u32>()) as GLsizeiptr,
                    shape.triangle_verticies.as_ptr() as *const c_void,
                    gl::STATIC_DRAW,
                );
                assert_eq!(gl::GetError(), 0);

                (vao, ebo, vbo, vbotx)
            };
            shape.compiled = Some(Arc::new(CompiledShape {
                vertex_array_object,
                entity_buffer_object,
                vertex_buffer_object,
                vertex_buffer_texture_coords_object,
                count: shape.triangle_verticies.len() as i32,
            }))
        }

        Ok(shape.compiled.as_ref().unwrap().clone())
    }
}

impl Default for Shape {
    fn default() -> Self {
        Shape {
            storage: Arc::new(RwLock::new(ShapeStorage::default())),
        }
    }
}

#[derive(Clone)]
pub struct Mesh {
    pub id: Entity,
    pub(crate) handle: MeshHandle,
}

#[derive(Clone)]
pub struct MeshHandle {
    pub(crate) storage: Arc<RwLock<MeshStorage>>,
}

pub(crate) struct MeshStorage {
    pub shape: Shape,
    pub material: Material,
    pub position: Point2d,
    pub scale: f32,
    pub angle: Rad<f32>,
    pub children: HashMap<Entity, Placement2d>,
}

pub mod prelude {
    pub use super::{Mesh, Scene2d, ScreenScene, Shape};
}

#[derive(Debug, Clone)]
pub(crate) struct Placement2d {
    pub id: Entity,
    pub position: Point2d,
    pub angle: Rad<f32>,
    pub scale: f32,
    pub location: Placement2dLocation,
}

#[derive(Debug, PartialEq, Clone)]
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
