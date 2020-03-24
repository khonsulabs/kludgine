use crate::{materials::prelude::*, runtime::flattened_scene::FlattenedMesh};
use cgmath::Matrix4;
use gl::types::*;
use std::ptr;

pub(crate) struct LoadedMesh {
    pub material: CompiledMaterial,
    pub vao: u32,
    pub ebo: u32,
    pub vbo: u32,
    pub count: i32,
    pub model: Matrix4<f32>,
    pub projection: Matrix4<f32>,
}

impl LoadedMesh {
    pub fn update(&mut self, mesh: &FlattenedMesh) {
        self.projection = mesh.projection;
        self.model = mesh.model;
    }

    pub fn compile(mesh: &FlattenedMesh) -> LoadedMesh {
        use std::mem;
        use std::os::raw::c_void;
        let (vao, ebo, vbo, material, count) = {
            let mesh = mesh.original.storage.lock().expect("Error locking mesh");
            let shape = mesh.shape.storage.lock().expect("Error locking shape");

            let (vao, ebo, vbo) = unsafe {
                let (mut vbo, mut vao, mut ebo) = (0, 0, 0);
                gl::GenVertexArrays(1, &mut vao);
                gl::GenBuffers(1, &mut vbo);
                gl::GenBuffers(1, &mut ebo);
                // bind the Vertex Array Object first, then bind and set vertex buffer(s), and then configure vertex attributes(s).
                gl::BindVertexArray(vao);

                gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (shape.vertices.len() * mem::size_of::<f32>() * 3) as GLsizeiptr,
                    shape.vertices.as_ptr() as *const c_void,
                    gl::STATIC_DRAW,
                );

                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
                gl::BufferData(
                    gl::ELEMENT_ARRAY_BUFFER,
                    (shape.triangles.len() * mem::size_of::<u32>() * 3) as GLsizeiptr,
                    shape.triangles.as_ptr() as *const c_void,
                    gl::STATIC_DRAW,
                );

                gl::VertexAttribPointer(
                    0,
                    3,
                    gl::FLOAT,
                    gl::FALSE,
                    3 * mem::size_of::<f32>() as GLsizei,
                    ptr::null(),
                );
                gl::EnableVertexAttribArray(0);

                // note that this is allowed, the call to gl::VertexAttribPointer registered VBO as the vertex attribute's bound vertex buffer object so afterwards we can safely unbind
                //gl::BindBuffer(gl::ARRAY_BUFFER, 0);

                // You can unbind the VAO afterwards so other VAO calls won't accidentally modify this VAO, but this rarely happens. Modifying other
                // VAOs requires a call to glBindVertexArray anyways so we generally don't unbind VAOs (nor VBOs) when it's not directly necessary.
                //gl::BindVertexArray(0);

                // uncomment this call to draw in wireframe polygons.
                // gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
                (vao, ebo, vbo)
            };

            let material = mesh.material.compile();

            (vao, ebo, vbo, material, shape.triangles.len() as i32 * 3)
        };

        LoadedMesh {
            vao,
            ebo,
            vbo,
            count,
            material,
            model: mesh.model,
            projection: mesh.projection,
        }
    }
}

impl Drop for LoadedMesh {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vbo);
            self.vbo = 0;
            gl::DeleteBuffers(1, &self.ebo);
            self.ebo = 0;
            gl::DeleteVertexArrays(1, &self.vao);
            self.vao = 0;
        }
    }
}
