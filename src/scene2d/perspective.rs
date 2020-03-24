use super::{Mesh2d, MeshStorage, Placement2d, Placement2dLocation, Scene2d, Shape};
use crate::internal_prelude::*;
use crate::materials::Material;
use std::sync::{Arc, Mutex};
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
}
