use crate::internal_prelude::*;
use cgmath::{prelude::*, Matrix4, Quaternion, Vector3};

pub struct FlattenedMesh {
    pub original: Mesh,
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
        let mut stack = scene
            .children
            .iter()
            .map(|(_, root)| {
                (
                    root.clone(),
                    scene.projection(&root.location),
                    Quaternion::<f32>::one(),
                    Vector3::new(
                        -scene.perspective_settings.camera_position.x,
                        -scene.perspective_settings.camera_position.y,
                        -scene.perspective_settings.camera_position.z,
                    ),
                    1.0f32,
                )
            })
            .collect::<Vec<_>>();

        while stack.len() > 0 {
            let mut new_stack = Vec::new();
            let flattened_meshes = stack
                .into_iter()
                .map(|(placement, projection, orientation, position, scale)| {
                    let mesh = scene.get(placement.id).unwrap();

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

                    // Do Operations that lock the mesh for reading
                    let material = {
                        let mesh = mesh.handle.storage.read().expect("Error locking mesh");
                        let material = mesh.material.clone();

                        for (_, placement) in mesh.children.iter() {
                            new_stack.push((
                                placement.clone(),
                                projection,
                                orientation,
                                position,
                                scale,
                            ));
                        }

                        material
                    };

                    FlattenedMesh {
                        original: mesh,
                        material,
                        projection,
                        model: translation
                            * Matrix4::from(orientation)
                            * Matrix4::from_scale(scale),
                        scale,
                    }
                })
                .collect::<Vec<_>>();
            self.meshes.extend(flattened_meshes);
            stack = new_stack;
        }
    }
}
