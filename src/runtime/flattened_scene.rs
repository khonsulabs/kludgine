use crate::scene2d::{Mesh2d, Scene2d};
use cgmath::{prelude::*, Matrix4, Quaternion, Vector3, Vector4};
use std::collections::HashMap;

#[derive(Default)]
pub struct FlattenedScene {
    pub meshes: Vec<FlattenedMesh2d>,
}

impl FlattenedScene {
    pub fn flatten_2d(&mut self, scene: &Scene2d) {
        // Loop over all placements and find all without relative_to's
        // Those objects are "roots", and we can start rendering the scene with those roots:
        // Render Z depth then Layers
        // Since all relative_to's must be on the same placement style (ie, you can't put a Screen-relative item relative to an object in Z-space)
        let mut placement_children: HashMap<
            Option<generational_arena::Index>,
            Vec<generational_arena::Index>,
        > = HashMap::new();
        for (_, v) in scene.placements.iter() {
            placement_children
                .entry(v.relative_to)
                .and_modify(|children| children.push(v.mesh.id))
                .or_insert_with(|| vec![v.mesh.id]);
        }

        let mut stack = Vec::new();
        for root_index in placement_children.get(&None).unwrap_or(&vec![]) {
            // Root starts out with their location's projection matrix, and then are modified from there. Unit-quaternion
            let root = scene.placements.get(root_index).unwrap();
            stack.push((
                root,
                scene.projection(&root.location),
                Quaternion::<f32>::one(),
                Vector4::<f32>::new(0.0, 0.0, 0.0, 0.0),
                1.0f32,
            ));
        }

        while let Some((placement, projection, orientation, position, scale)) = stack.pop() {
            let mesh = placement.mesh.clone();

            let mesh_position = orientation.rotate_vector(Vector3::new(
                placement.position.x,
                placement.position.y,
                0.0,
            ));

            let position = position
                + Vector4::new(
                    mesh_position.x * scale,
                    mesh_position.y * scale,
                    mesh_position.z * scale,
                    0.0,
                );
            let offset = projection * position;
            let orientation = orientation * Quaternion::from_angle_z(placement.angle);
            let scale = scale * placement.scale;

            self.meshes.push(FlattenedMesh2d {
                mesh,
                projection,
                model: Matrix4::from(orientation) * Matrix4::from_scale(scale),
                offset,
                scale,
            });
            //  Push all children
            if let Some(children) = placement_children.get(&Some(placement.mesh.id)) {
                for child_index in children.iter() {
                    let placement = scene.placements.get(child_index).unwrap();

                    stack.push((placement, projection, orientation, position, scale));
                }
            }
        }
    }
}

pub struct FlattenedMesh2d {
    pub mesh: Mesh2d,
    pub projection: Matrix4<f32>,
    pub model: Matrix4<f32>,
    pub offset: Vector4<f32>,
    pub scale: f32,
}
