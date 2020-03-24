use crate::internal_prelude::*;
use cgmath::Vector4;
use gl::types::*;
use std::{
    ffi::CString,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug)]
pub enum MaterialKind {
    Solid { color: Color },
}

pub(crate) struct MaterialStorage {
    pub kind: MaterialKind,
    compiled: Option<CompiledMaterial>,
}

#[derive(Clone)]
pub struct Material {
    storage: Arc<Mutex<MaterialStorage>>,
}

impl From<MaterialKind> for Material {
    fn from(kind: MaterialKind) -> Self {
        let storage = Arc::new(Mutex::new(MaterialStorage {
            kind,
            compiled: None,
        }));
        Self { storage }
    }
}

const VERTEX_SHADER_SOURCE: &str = r#"
    #version 140
    uniform mat4 projection;
    uniform mat4 model;
    in vec3 position;
    void main() {
        vec4 transformed = model * vec4(position, 1.0);

        gl_Position = projection * transformed;
    }
"#;

const FRAGMENT_SHADER_SOURCE: &str = r#"
    #version 140
    uniform vec4 color;
    out vec4 f_color;
    void main() {
        f_color = color;
    }
"#;

impl Material {
    pub(crate) fn compile(&self) -> CompiledMaterial {
        let mut storage = self.storage.lock().expect("Error locking material");
        if let None = &storage.compiled {
            match &storage.kind {
                MaterialKind::Solid { color } => {
                    use std::ptr;
                    use std::str;
                    let shader_program = unsafe {
                        // build and compile our shader program
                        // ------------------------------------
                        // vertex shader
                        let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
                        let c_str_vert = CString::new(VERTEX_SHADER_SOURCE.as_bytes()).unwrap();
                        gl::ShaderSource(vertex_shader, 1, &c_str_vert.as_ptr(), ptr::null());
                        gl::CompileShader(vertex_shader);

                        // check for shader compile errors
                        let mut success = gl::FALSE as GLint;
                        let mut info_log = Vec::with_capacity(512);
                        info_log.set_len(512 - 1); // subtract 1 to skip the trailing null character
                        gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut success);
                        if success != gl::TRUE as GLint {
                            gl::GetShaderInfoLog(
                                vertex_shader,
                                512,
                                ptr::null_mut(),
                                info_log.as_mut_ptr() as *mut GLchar,
                            );
                            println!(
                                "ERROR::SHADER::VERTEX::COMPILATION_FAILED\n{}",
                                str::from_utf8(&info_log).unwrap()
                            );
                        }

                        // fragment shader
                        let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
                        let c_str_frag = CString::new(FRAGMENT_SHADER_SOURCE.as_bytes()).unwrap();
                        gl::ShaderSource(fragment_shader, 1, &c_str_frag.as_ptr(), ptr::null());
                        gl::CompileShader(fragment_shader);
                        // check for shader compile errors
                        gl::GetShaderiv(fragment_shader, gl::COMPILE_STATUS, &mut success);
                        if success != gl::TRUE as GLint {
                            gl::GetShaderInfoLog(
                                fragment_shader,
                                512,
                                ptr::null_mut(),
                                info_log.as_mut_ptr() as *mut GLchar,
                            );
                            println!(
                                "ERROR::SHADER::FRAGMENT::COMPILATION_FAILED\n{}",
                                str::from_utf8(&info_log).unwrap()
                            );
                        }

                        // link shaders
                        let shader_program = gl::CreateProgram();
                        gl::AttachShader(shader_program, vertex_shader);
                        gl::AttachShader(shader_program, fragment_shader);
                        gl::LinkProgram(shader_program);
                        // check for linking errors
                        gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);
                        if success != gl::TRUE as GLint {
                            gl::GetProgramInfoLog(
                                shader_program,
                                512,
                                ptr::null_mut(),
                                info_log.as_mut_ptr() as *mut GLchar,
                            );
                            println!(
                                "ERROR::SHADER::PROGRAM::COMPILATION_FAILED\n{}",
                                str::from_utf8(&info_log).unwrap()
                            );
                        }
                        gl::DeleteShader(vertex_shader);
                        gl::DeleteShader(fragment_shader);

                        shader_program
                    };

                    storage.compiled = Some(CompiledMaterial {
                        shader_program,
                        color: Vector4::new(
                            color.red as f32 / 255.0,
                            color.blue as f32 / 255.0,
                            color.green as f32 / 255.0,
                            color.alpha as f32 / 255.0,
                        ),
                    });
                }
            }
        }

        storage.compiled.as_ref().unwrap().clone()
    }
}

#[derive(Clone)]
pub(crate) struct CompiledMaterial {
    pub shader_program: u32,
    pub color: Vector4<f32>,
}

impl CompiledMaterial {
    pub(crate) fn activate(&self) {
        unsafe {
            gl::UseProgram(self.shader_program);
            gl::Uniform4f(
                gl::GetUniformLocation(
                    self.shader_program,
                    CString::new("color".as_bytes()).unwrap().as_ptr(),
                ),
                self.color.x,
                self.color.y,
                self.color.z,
                self.color.w,
            );
            if self.color.w < 1.0 {
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            } else {
                gl::Disable(gl::BLEND);
            }
        }
    }
}

impl Drop for CompiledMaterial {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.shader_program);
            self.shader_program = 0;
        }
    }
}
