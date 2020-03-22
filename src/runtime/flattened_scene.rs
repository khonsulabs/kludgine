use crate::material::Material;
use crate::scene2d::{Mesh2d, Placement2dLocation, Scene2d};
use cgmath::{prelude::*, Matrix4, Point3, Quaternion};
use std::cmp::Ordering;
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
        for (k, v) in scene.placements.iter() {
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
                Point3::<f32>::origin(),
            ));
        }

        while let Some((placement, projection, orientation, position)) = stack.pop() {
            self.meshes.push(FlattenedMesh2d {
                mesh: placement.mesh.clone(),
                projection,
                orientation,
                position,
            });
            // Flatten this mesh
            //   * Translate position relatative to parent or 0,0
            //   * Append Z rotation quaternion
            //  Push all children
            if let Some(children) = placement_children.get(&Some(placement.mesh.id)) {
                for child_index in children.iter() {
                    let placement = scene.placements.get(child_index).unwrap();

                    stack.push((placement, projection, orientation, position));
                }
            }
        }
    }
}

pub struct FlattenedMesh2d {
    pub mesh: Mesh2d,
    pub projection: Matrix4<f32>,
    pub orientation: Quaternion<f32>,
    pub position: Point3<f32>,
}
