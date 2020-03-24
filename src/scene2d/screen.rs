use super::{Mesh, MeshStorage, Placement2d, Placement2dLocation, Scene2d, Shape};
use crate::internal_prelude::*;
use crate::materials::Material;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
pub struct ScreenScene<'a> {
    pub scene: &'a mut Scene2d,
}

impl<'a> ScreenScene<'a> {
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
        let id = self.scene.arena.insert(storage.clone());
        Mesh { id, storage }
    }

    pub fn create_mesh_clone(&mut self, copy: &Mesh) -> Mesh {
        let copy_storage = copy.storage.read().expect("Error locking copy storage");
        let storage = Arc::new(RwLock::new(MeshStorage {
            shape: copy_storage.shape.clone(),
            material: copy_storage.material.clone(),
            angle: copy_storage.angle,
            scale: copy_storage.scale,
            position: copy_storage.position,
            children: copy_storage.children.clone(), // TODO Do a deep clone so
        }));
        let id = self.scene.arena.insert(storage.clone());
        Mesh { id, storage }
    }

    pub fn place_mesh(
        &mut self,
        mesh: &Mesh,
        relative_to: Option<generational_arena::Index>,
        position: Point2d,
        angle: Rad<f32>,
        scale: f32,
    ) -> KludgineResult<()> {
        match relative_to {
            Some(relative_to) => match self.scene.get(relative_to) {
                Some(relative_mesh) => {
                    let mut storage = relative_mesh
                        .storage
                        .write()
                        .expect("Error locking mesh for writing");
                    Self::internal_place_mesh(&mut storage.children, &mesh, position, angle, scale)
                }
                None => Err(KludgineError::InvalidId(relative_to)),
            },
            None => {
                Self::internal_place_mesh(&mut self.scene.children, mesh, position, angle, scale)
            }
        }
    }

    fn internal_place_mesh(
        children: &mut HashMap<generational_arena::Index, Placement2d>,
        mesh: &Mesh,
        position: Point2d,
        angle: Rad<f32>,
        scale: f32,
    ) -> KludgineResult<()> {
        children
            .entry(mesh.id)
            .and_modify(|p| {
                p.position = position;
                p.angle = angle;
                p.scale = scale;
                p.location = Placement2dLocation::Screen;
            })
            .or_insert_with(|| Placement2d {
                id: mesh.id,
                position,
                angle,
                scale,
                location: Placement2dLocation::Screen,
            });
        Ok(())
    }
}
