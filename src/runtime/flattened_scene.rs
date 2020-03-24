use crate::internal_prelude::*;
use cgmath::{prelude::*, Matrix4, Quaternion, Vector3};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::mpsc::channel;

pub struct FlattenedMesh {
    pub original: Mesh2d,
    pub material: Material,
    pub projection: Matrix4<f32>,
    pub model: Matrix4<f32>,
    pub scale: f32,
}

#[derive(Default)]
pub struct FlattenedScene {
    pub meshes: Vec<FlattenedMesh>,
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

        let mut stack = placement_children
            .get(&None)
            .unwrap_or(&vec![])
            .into_par_iter()
            .map(|root_index| {
                let root = scene.placements.get(root_index).unwrap();
                (
                    root,
                    scene.projection(&root.location),
                    Quaternion::<f32>::one(),
                    Vector3::<f32>::new(0.0, 0.0, 0.0),
                    1.0f32,
                )
            })
            .collect::<Vec<_>>();

        while stack.len() > 0 {
            let (sender, receiver) = channel();
            let flattened_meshes = stack
                .into_par_iter()
                .map_with(
                    sender,
                    |sender, (placement, projection, orientation, position, scale)| {
                        let mesh = placement.mesh.clone();

                        let mesh_position = orientation.rotate_vector(Vector3::new(
                            placement.position.x,
                            placement.position.y,
                            placement.z(),
                        ));

                        let position = position
                            + Vector3::new(
                                mesh_position.x * scale,
                                mesh_position.y * scale,
                                mesh_position.z * scale,
                            );
                        let translation = Matrix4::from_translation(position);
                        let orientation = orientation * Quaternion::from_angle_z(placement.angle);
                        let scale = scale * placement.scale;

                        let material = {
                            let mesh = placement.mesh.storage.lock().expect("Error locking mesh");
                            let material = mesh.material.clone();

                            material
                        };

                        //  Push all children
                        if let Some(children) = placement_children.get(&Some(placement.mesh.id)) {
                            for child_index in children.iter() {
                                let placement = scene.placements.get(child_index).unwrap();

                                sender
                                    .send((placement, projection, orientation, position, scale))
                                    .unwrap();
                            }
                        }

                        FlattenedMesh {
                            original: mesh,
                            material,
                            projection,
                            model: translation
                                * Matrix4::from(orientation)
                                * Matrix4::from_scale(scale),
                            scale,
                        }
                    },
                )
                .collect::<Vec<_>>();
            self.meshes.extend(flattened_meshes);
            stack = receiver.iter().collect();
        }
    }
}
