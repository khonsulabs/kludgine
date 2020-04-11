use crate::{
    internal_prelude::*,
    materials::material::SimpleMaterial,
    shaders::{CompiledProgram, Program, ProgramSource},
    texture::CompiledTexture,
};

const VERTEX_SHADER_SOURCE: &str = r#"
    #version 330 core
    layout(location=0) in vec3 in_position;
    layout(location=1) in vec2 in_tex_coord;
    out vec2 TexCoord;
    uniform mat4 projection;
    uniform mat4 model;
    void main() {
        vec4 transformed = model * vec4(in_position, 1.0);
        gl_Position = projection * transformed;
        TexCoord = in_tex_coord;
    }
"#;

const FRAGMENT_SHADER_SOURCE: &str = r#"
    #version 330 core
    uniform sampler2D uniformTexture;

    in vec2 TexCoord;

    out vec4 FragmentColor;

    void main() {
        FragmentColor = texture(uniformTexture, TexCoord);
    }
"#;

pub(crate) struct TexturedMaterial {
    texture: Arc<CompiledTexture>,
}

pub(crate) fn simple_material(texture: Arc<CompiledTexture>) -> KludgineHandle<dyn SimpleMaterial> {
    KludgineHandle::wrap(TexturedMaterial { texture })
}

impl SimpleMaterial for TexturedMaterial {
    fn program(&self) -> KludgineResult<Program> {
        Ok(ProgramSource {
            vertex_shader: Some(VERTEX_SHADER_SOURCE.to_owned()),
            fragment_shader: Some(FRAGMENT_SHADER_SOURCE.to_owned()),
        }
        .into())
    }
    fn activate(&self, program: &CompiledProgram) -> KludgineResult<()> {
        program.set_uniform_1i("uniformTexture", 0);
        self.texture.activate();
        Ok(())
    }
}
