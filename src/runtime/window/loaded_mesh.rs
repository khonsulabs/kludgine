use crate::{
    internal_prelude::*, materials::prelude::*, runtime::flattened_scene::FlattenedMesh,
    scene2d::CompiledShape,
};
use cgmath::Matrix4;
use std::{ptr, sync::Arc};

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

    pub fn compile(mesh: &FlattenedMesh) -> KludgineResult<LoadedMesh> {
        let (shape, material) = {
            let mesh = mesh.original.storage.read().expect("Error locking mesh");
            let material = mesh.material.compile()?;
            let shape = mesh.shape.compile()?;
            (shape, material)
        };

        Ok(LoadedMesh {
            shape,
            material,
            model: mesh.model,
            projection: mesh.projection,
        })
    }

    pub fn activate(&self) {
        self.material.activate();
        self.shape.activate();
    }

    pub fn render(&self) {
        self.activate();
        self.material
            .program
            .set_uniform_matrix4f("projection", &self.projection);
        self.material
            .program
            .set_uniform_matrix4f("model", &self.model);
        unsafe {
            gl::DrawElements(
                gl::TRIANGLES,
                self.shape.count,
                gl::UNSIGNED_INT,
                ptr::null(),
            );
        }
    }
}
