use super::{Mesh, Placement2d, Placement2dLocation, Scene2d};
use crate::internal_prelude::*;
use std::collections::HashMap;
pub struct PerspectiveScene<'a> {
    pub scene: &'a mut Scene2d,
}

impl<'a> PerspectiveScene<'a> {
    pub fn place_mesh(
        &mut self,
        mesh: &Mesh,
        relative_to: Option<generational_arena::Index>,
        position: Point3d,
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
        position: Point3d,
        angle: Rad<f32>,
        scale: f32,
    ) -> KludgineResult<()> {
        children
            .entry(mesh.id)
            .and_modify(|p| {
                p.position = Point2d::new(position.x, position.y);
                p.angle = angle;
                p.scale = scale;
                p.location = Placement2dLocation::Z(position.z);
            })
            .or_insert_with(|| Placement2d {
                id: mesh.id,
                position: Point2d::new(position.x, position.y),
                angle,
                scale,
                location: Placement2dLocation::Z(position.z),
            });

        Ok(())
    }

    pub fn set_fov<F: Into<Deg<f32>>>(&mut self, fov: F) {
        self.scene.perspective_settings.fov = fov.into();
    }

    pub fn fov(&self) -> Deg<f32> {
        self.scene.perspective_settings.fov
    }

    pub fn set_zrange(&mut self, znear: f32, zfar: f32) {
        assert!(znear > 0.0);
        assert!(zfar > znear);
        self.scene.perspective_settings.znear = znear;
        self.scene.perspective_settings.zfar = zfar;
    }

    pub fn znear(&self) -> f32 {
        self.scene.perspective_settings.znear
    }

    pub fn zfar(&self) -> f32 {
        self.scene.perspective_settings.zfar
    }
}
