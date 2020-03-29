use crate::{
    internal_prelude::*,
    materials::material::SimpleMaterial,
    shaders::{CompiledProgram, Program, ProgramSource},
};
use cgmath::Vector4;

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

pub(crate) fn program() -> Program {
    ProgramSource {
        vertex_shader: Some(VERTEX_SHADER_SOURCE.to_owned()),
        fragment_shader: Some(FRAGMENT_SHADER_SOURCE.to_owned()),
    }
    .into()
}

struct SolidMaterial {
    color: Vector4<f32>,
}

pub(crate) fn simple_material(color: Vector4<f32>) -> Box<dyn SimpleMaterial> {
    Box::new(SolidMaterial { color })
}

impl SimpleMaterial for SolidMaterial {
    fn program(&self) -> KludgineResult<Program> {
        Ok(program())
    }
    fn activate(&self, program: &CompiledProgram) -> KludgineResult<()> {
        program.set_uniform_vec4("color", &self.color);
        if self.color.w < 1.0 {
            unsafe {
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            }
        } else {
            unsafe {
                gl::Disable(gl::BLEND);
            }
        }
        Ok(())
    }
}
