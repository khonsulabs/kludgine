use crate::internal_prelude::*;
use cgmath::{Matrix4, Vector4};
use gl::types::*;
use std::{ffi::CString, ptr, str};

#[derive(Clone)]
pub struct Program {
    pub(crate) storage: KludgineHandle<ProgramStorage>,
}

#[derive(Default)]
pub struct ProgramSource {
    pub vertex_shader: Option<String>,
    pub fragment_shader: Option<String>,
}

impl From<ProgramSource> for Program {
    fn from(source: ProgramSource) -> Self {
        Self {
            storage: KludgineHandle::wrap(ProgramStorage {
                source,
                compiled: None,
            }),
        }
    }
}

fn compile_shader(gl_kind: u32, src: &str) -> KludgineResult<u32> {
    unsafe {
        let shader = gl::CreateShader(gl_kind);
        let c_str_vert = CString::new(src.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &c_str_vert.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        // check for shader compile errors
        let mut success = 0i32;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        if success == gl::TRUE as i32 {
            Ok(shader)
        } else {
            let mut info_length = 0i32;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut info_length);
            let info_length = info_length as usize;
            let mut info_log = Vec::with_capacity(info_length);
            info_log.resize(info_length, 0);
            gl::GetShaderInfoLog(
                shader,
                info_length as i32,
                ptr::null_mut::<i32>(),
                info_log.as_mut_ptr() as *mut i8,
            );
            info_log.truncate(info_log.len() - 1);
            let message = CString::new(info_log).unwrap();
            let message = message
                .to_str()
                .expect("Invalid UTF-8 characters in info_log");
            Err(KludgineError::ShaderCompilationError(message.to_owned()))
        }
    }
}

impl Program {
    pub(crate) fn compile(&self) -> KludgineResult<Arc<CompiledProgram>> {
        {
            let storage = self.storage.read().expect("Error locking program for read");
            if let Some(program) = &storage.compiled {
                return Ok(program.clone());
            }
        }

        let mut storage = self
            .storage
            .write()
            .expect("Error locking program for write");

        let vertex_shader = match &storage.source.vertex_shader {
            Some(source) => Some(compile_shader(gl::VERTEX_SHADER, source)?),
            None => None,
        };
        let fragment_shader = match &storage.source.fragment_shader {
            Some(source) => Some(compile_shader(gl::FRAGMENT_SHADER, source)?),
            None => None,
        };

        let program = unsafe {
            // link shaders
            let shader_program = gl::CreateProgram();
            if let Some(vertex_shader) = &vertex_shader {
                gl::AttachShader(shader_program, *vertex_shader);
            }
            if let Some(fragment_shader) = &fragment_shader {
                gl::AttachShader(shader_program, *fragment_shader);
            }

            gl::LinkProgram(shader_program);

            if let Some(vertex_shader) = &vertex_shader {
                gl::DeleteShader(*vertex_shader);
            }
            if let Some(fragment_shader) = &fragment_shader {
                gl::DeleteShader(*fragment_shader);
            }

            // check for linking errors
            let mut success = 0i32;
            gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);
            if success == gl::TRUE as GLint {
                shader_program
            } else {
                let mut info_length = 0i32;
                gl::GetProgramiv(shader_program, gl::INFO_LOG_LENGTH, &mut info_length);
                let mut info_log = Vec::with_capacity((info_length + 1) as usize);
                gl::GetProgramInfoLog(
                    shader_program,
                    info_length + 1,
                    ptr::null_mut::<i32>(),
                    info_log.as_mut_ptr() as *mut i8,
                );
                let message = CString::new(info_log).unwrap();
                let message = message
                    .to_str()
                    .expect("Invalid UTF-8 characters in info_log");
                return Err(KludgineError::ShaderCompilationError(message.to_owned()));
            }
        };

        let compiled = Arc::new(CompiledProgram { program });
        storage.compiled = Some(compiled.clone());
        Ok(compiled)
    }
}

#[derive(Default)]
pub(crate) struct ProgramStorage {
    source: ProgramSource,
    compiled: Option<Arc<CompiledProgram>>,
}

#[derive(Clone, Debug)]
pub struct CompiledProgram {
    pub program: u32,
}

impl CompiledProgram {
    pub fn activate(&self) {
        unsafe {
            gl::UseProgram(self.program);
        }
    }

    pub fn set_uniform_1i(&self, name: &str, value: i32) {
        unsafe {
            gl::Uniform1i(
                gl::GetUniformLocation(
                    self.program,
                    CString::new(name.as_bytes()).unwrap().as_ptr(),
                ),
                value,
            );
        }
    }

    pub fn set_uniform_vec4(&self, name: &str, vector: &Vector4<f32>) {
        unsafe {
            gl::Uniform4f(
                gl::GetUniformLocation(
                    self.program,
                    CString::new(name.as_bytes()).unwrap().as_ptr(),
                ),
                vector.x,
                vector.y,
                vector.z,
                vector.w,
            );
        }
    }

    pub fn set_uniform_matrix4f(&self, name: &str, matrix: &Matrix4<f32>) {
        unsafe {
            gl::UniformMatrix4fv(
                gl::GetUniformLocation(
                    self.program,
                    CString::new(name.as_bytes()).unwrap().as_ptr(),
                ),
                1,
                gl::FALSE,
                matrix.as_ptr() as *const f32,
            );
        }
    }
}

impl Drop for CompiledProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.program);
            self.program = 0;
        }
    }
}
