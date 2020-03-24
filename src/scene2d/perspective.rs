use super::{Mesh2d, Placement2d, Placement2dLocation, Scene2d};
use crate::internal_prelude::*;
pub struct PerspectiveScene<'a> {
    pub scene: &'a mut Scene2d,
}

impl<'a> PerspectiveScene<'a> {
    pub fn place_mesh(
        &mut self,
        mesh: &Mesh2d,
        relative_to: Option<generational_arena::Index>,
        position: Point3d,
        angle: Rad<f32>,
        scale: f32,
    ) {
        self.scene
            .placements
            .entry(mesh.id)
            .and_modify(|p| {
                p.relative_to = relative_to;
                p.position = Point2d::new(position.x, position.y);
                p.angle = angle;
                p.scale = scale;
                p.location = Placement2dLocation::Z(position.z);
            })
            .or_insert_with(|| Placement2d {
                mesh: mesh.clone(),
                relative_to,
                position: Point2d::new(position.x, position.y),
                angle,
                scale,
                location: Placement2dLocation::Z(position.z),
            });
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
