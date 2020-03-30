use kludgine::prelude::*;
use std::sync::{Arc, RwLock};
use rand::{Rng, thread_rng};

fn main() {
    Runtime::new(SingleWindowApplication::<ShaderWindow>::default()).run();
}

struct ShaderWindow {
    material: Arc<RwLock<CustomShaderMaterial>>,
}

impl WindowCreator<ShaderWindow> for ShaderWindow {
    fn window_title() -> String {
        "Custom Shaders - Kludgine".to_owned()
    }
}

impl Default for ShaderWindow {
    fn default() -> Self {
        Self {
            material: Arc::new(RwLock::new(CustomShaderMaterial {
                color_one: Vector4::new(1.0, 0.0, 0.0, 1.0),
                color_two: Vector4::new(0.0, 1.0, 0.0, 1.0),
                frame: 0,
            })),
        }
    }
}

struct CustomShaderMaterial {
    color_one: Vector4<f32>,
    color_two: Vector4<f32>,
    frame: i32,
}

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
    uniform vec4 color_one;
    uniform vec4 color_two;
    uniform int frame;
    out vec4 f_color;
    void main() {
        float tween = float(frame) / 144.0;
        f_color = color_one * (1.0f - tween) + color_two * tween;
    }
"#;

impl SimpleMaterial for CustomShaderMaterial {
    fn program(&self) -> KludgineResult<Program> {
        Ok(ProgramSource {
            fragment_shader: Some(FRAGMENT_SHADER_SOURCE.into()),
            vertex_shader: Some(VERTEX_SHADER_SOURCE.into()),
        }.into())
    }
    fn activate(&self, program: &CompiledProgram) -> KludgineResult<()> {
        program.set_uniform_vec4("color_one", &self.color_one);
        program.set_uniform_vec4("color_two", &self.color_two);
        program.set_uniform_1i("frame", self.frame);
        Ok(())
    }
}

#[async_trait]
impl Window for ShaderWindow {
    async fn render_2d(&mut self, scene: &mut Scene2d) -> KludgineResult<()> {
        let mesh = scene.cached_mesh("moon", |scene| {
            let material = self.material.clone();
            let shape = Shape::rect(&Rect::new(Point2d::new(-0.5, -0.5), Size2d::new(1.0, 1.0)));
            Ok(scene.create_mesh(
                shape,
                material,
            ))
        }).unwrap();

        {
            let mut material = self.material.write().expect("Error locking material for write");
            material.frame += 1;
            if material.frame >= 144 {
                let mut rng = thread_rng();
                material.color_one = material.color_two;
                material.color_two = Vector4::new(rng.gen(), rng.gen(), rng.gen(), 1.0);
                material.frame = 0;
            }
        }
        
        scene.perspective().place_mesh(
            &mesh,
            None,
            Point3d::new(0.0, 0.0, -2.0),
            Deg(0.0).into(),
            1.0,
        )?;
        Ok(())
    }
}
