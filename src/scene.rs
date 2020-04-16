use super::math::Size;
use legion::prelude::*;

use std::collections::HashSet;
use winit::event::VirtualKeyCode;

pub struct Scene {
    pub universe: Universe,
    pub world: World,
    pub pressed_keys: HashSet<VirtualKeyCode>,
    pub(crate) scale_factor: f32,
    pub(crate) size: Size,
}

impl Scene {
    pub fn new() -> Self {
        let universe = Universe::new();
        let world = universe.create_world();
        Self {
            universe,
            world,
            scale_factor: 1.0,
            size: Size::default(),
            pressed_keys: HashSet::new(),
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    // pub fn get(&self, id: Entity) -> Option<Mesh> {
    //     match self.world.get_component::<MeshHandle>(id) {
    //         Some(handle) => Some(Mesh {
    //             id,
    //             handle: handle.as_ref().clone(),
    //         }),
    //         None => None,
    //     }
    // }

    // pub fn cached_mesh<S: Into<String>, F: FnOnce(&mut Scene2d) -> KludgineResult<Mesh>>(
    //     &mut self,
    //     name: S,
    //     initializer: F,
    // ) -> KludgineResult<Mesh> {
    //     let name = name.into();
    //     match self.lazy_mesh_cache.get(&name) {
    //         Some(mesh) => Ok(mesh.clone()),
    //         None => {
    //             let new_mesh = initializer(self)?;
    //             self.lazy_mesh_cache.insert(name, new_mesh.clone());
    //             Ok(new_mesh)
    //         }
    //     }
    // }

    // pub fn create_mesh<M: Into<Material>>(&mut self, shape: Shape, material: M) -> Mesh {
    //     let material = material.into();
    //     let storage = KludgineHandle::wrap(MeshStorage {
    //         shape,
    //         material,
    //         angle: Rad(0.0),
    //         scale: 1.0,
    //         position: Point2d::new(0.0, 0.0),
    //         children: HashMap::new(),
    //     });
    //     let handle = MeshHandle { storage };
    //     let id = self.world.insert((), vec![(handle.clone(),)])[0];
    //     Mesh { id, handle }
    // }
}

pub(crate) struct FlattenedScene {}
