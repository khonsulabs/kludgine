use super::{Mesh2d, MeshStorage, Placement2d, Placement2dLocation, Scene2d, Shape};
use crate::internal_prelude::*;
use crate::materials::Material;
use std::sync::{Arc, RwLock};
pub struct ScreenScene<'a> {
    pub scene: &'a mut Scene2d,
}

impl<'a> ScreenScene<'a> {
    pub fn create_mesh<M: Into<Material>>(&mut self, shape: Shape, material: M) -> Mesh2d {
        let material = material.into();
        let storage = Arc::new(RwLock::new(MeshStorage {
            shape,
            material,
            angle: Rad(0.0),
            scale: 1.0,
            position: Point2d::new(0.0, 0.0),
        }));
        let id = self.scene.arena.insert(storage.clone());
        Mesh2d { id, storage }
    }

    pub fn create_mesh_clone(&mut self, copy: &Mesh2d) -> Mesh2d {
        let copy_storage = copy.storage.read().expect("Error locking copy storage");
        let storage = Arc::new(RwLock::new(MeshStorage {
            shape: copy_storage.shape.clone(),
            material: copy_storage.material.clone(),
            angle: copy_storage.angle,
            scale: copy_storage.scale,
            position: copy_storage.position,
        }));
        let id = self.scene.arena.insert(storage.clone());
        Mesh2d { id, storage }
    }

    pub fn place_mesh(
        &mut self,
        mesh: &Mesh2d,
        relative_to: Option<generational_arena::Index>,
        position: Point2d,
        angle: Rad<f32>,
        scale: f32,
    ) {
        self.scene
            .placements
            .entry(mesh.id)
            .and_modify(|p| {
                p.relative_to = relative_to;
                p.position = position;
                p.angle = angle;
                p.scale = scale;
                p.location = Placement2dLocation::Screen;
            })
            .or_insert_with(|| Placement2d {
                mesh: mesh.clone(),
                relative_to,
                position,
                angle,
                scale,
                location: Placement2dLocation::Screen,
            });
    }
}
