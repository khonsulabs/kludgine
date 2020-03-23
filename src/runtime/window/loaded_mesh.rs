use crate::internal_prelude::*;
use crate::{materials::prelude::*, runtime::flattened_scene::FlattenedMesh2d};
use cgmath::{Matrix4, Vector4};
use gl::types::*;
use std::ptr;

pub(crate) struct LoadedMesh {
    pub material: CompiledMaterial,
    pub position: Vector4<f32>,
    pub vao: u32,
    pub ebo: u32,
    pub vbo: u32,
    pub count: i32,
    pub projection: Matrix4<f32>,
    pub model: Matrix4<f32>,
}

impl LoadedMesh {
    pub fn compile(mesh: &FlattenedMesh2d) -> LoadedMesh {
        use std::mem;
        use std::os::raw::c_void;
        let (vao, ebo, vbo, material, count) = {
            let storage = mesh.mesh.storage.lock().expect("Error locking mesh");
            let shape = storage.shape.storage.lock().expect("Error locking shape");
            let vertices: &[Point2d] = &shape.vertices;
            let faces = shape
                .triangles
                .iter()
                .map(|(a, b, c)| (a.0, b.0, c.0))
                .collect::<Vec<(u32, u32, u32)>>();
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
                    (vertices.len() * mem::size_of::<f32>() * 2) as GLsizeiptr,
                    vertices.as_ptr() as *const c_void,
                    gl::STATIC_DRAW,
                );

                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
                gl::BufferData(
                    gl::ELEMENT_ARRAY_BUFFER,
                    (faces.len() * mem::size_of::<u32>() * 3) as GLsizeiptr,
                    faces.as_ptr() as *const c_void,
                    gl::STATIC_DRAW,
                );

                gl::VertexAttribPointer(
                    0,
                    2,
                    gl::FLOAT,
                    gl::FALSE,
                    2 * mem::size_of::<f32>() as GLsizei,
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

            let material = storage.material.compile();

            (vao, ebo, vbo, material, faces.len() as i32 * 3)
        };

        LoadedMesh {
            vao,
            ebo,
            vbo,
            count,
            material,
            position: mesh.offset,
            projection: mesh.projection,
            model: mesh.model,
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
