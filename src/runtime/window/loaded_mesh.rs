use crate::{
    materials::prelude::*, runtime::flattened_scene::FlattenedMesh, scene2d::CompiledShape,
};
use cgmath::Matrix4;
use std::sync::Arc;

pub(crate) struct LoadedMesh {
    pub material: Arc<CompiledMaterial>,
    pub shape: Arc<CompiledShape>,
    pub model: Matrix4<f32>,
    pub projection: Matrix4<f32>,
}

impl LoadedMesh {
    pub fn update(&mut self, mesh: &FlattenedMesh) {
        self.projection = mesh.projection;
        self.model = mesh.model;
    }

    pub fn compile(mesh: &FlattenedMesh) -> LoadedMesh {
        let (shape, material) = {
            let mesh = mesh.original.storage.read().expect("Error locking mesh");
            let material = mesh.material.compile();
            let shape = mesh.shape.compile();
            (shape, material)
        };

        LoadedMesh {
            shape,
            material,
            model: mesh.model,
            projection: mesh.projection,
        }
    }

    pub fn activate(&self) {
        self.material.activate();
        self.shape.activate();
    }
}
