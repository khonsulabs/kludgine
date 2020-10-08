use crate::math::{Angle, Point, Raw};
use easygpu::{
    color::Rgba8,
    core::{self},
    transform::{ScreenSpace, ScreenTransformation},
};
use euclid::{Vector2D, Vector3D};

/// A pipeline for rendering shapes.
pub struct Pipeline {
    pipeline: core::Pipeline,
    bindings: core::BindingGroup,
    buf: core::UniformBuffer,
}

#[repr(C)]
#[derive(Copy, Clone)]
/// The uniforms for the shader. These uniforms match those from rgx's built-in pipelines, and the math performed is identical
pub struct Uniforms {
    /// The orthographic projection matrix
    pub ortho: ScreenTransformation<f32>,
    /// The transformation matrix
    pub transform: ScreenTransformation<f32>,
}
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: Vector3D<f32, ScreenSpace>,
    pub uv: Vector2D<f32, ScreenSpace>,
    pub color: Rgba8,
}

impl Vertex {
    pub fn rotate_by(mut self, angle: Option<Angle>, origin: Point<f32, Raw>) -> Self {
        if let Some(angle) = angle {
            let origin = origin.to_vector();
            let rotation2d = euclid::Rotation2D::new(angle);
            let position = Point::new(self.position.x, self.position.y);
            let relative_position = position - origin;
            let rotated = rotation2d.transform_point(relative_position) + origin;

            self.position.x = rotated.x;
            self.position.y = rotated.y;

            self
        } else {
            self
        }
    }
}

impl Pipeline {
    pub fn binding(
        &self,
        renderer: &core::Renderer,
        texture: &core::Texture,
        sampler: &core::Sampler,
    ) -> core::BindingGroup {
        renderer
            .device
            .create_binding_group(&self.pipeline.layout.sets[1], &[texture, sampler])
    }
}

impl<'a> core::AbstractPipeline<'a> for Pipeline {
    type PrepareContext = ScreenTransformation<f32>;
    type Uniforms = self::Uniforms;

    fn description() -> core::PipelineDescription<'a> {
        core::PipelineDescription {
            vertex_layout: &[
                core::VertexFormat::Float3,
                core::VertexFormat::Float2,
                core::VertexFormat::UByte4,
            ],
            pipeline_layout: &[
                core::Set(&[easygpu::core::Binding {
                    binding: easygpu::core::BindingType::UniformBuffer,
                    stage: easygpu::core::ShaderStage::Vertex,
                }]),
                core::Set(&[
                    core::Binding {
                        binding: core::BindingType::SampledTexture,
                        stage: core::ShaderStage::Fragment,
                    },
                    core::Binding {
                        binding: core::BindingType::Sampler,
                        stage: core::ShaderStage::Fragment,
                    },
                ]),
            ],
            vertex_shader: include_bytes!("shaders/sprite.vert.spv"),
            fragment_shader: include_bytes!("shaders/sprite.frag.spv"),
        }
    }

    fn setup(pipeline: core::Pipeline, dev: &core::Device) -> Self {
        let transform = ScreenTransformation::identity();
        let ortho = ScreenTransformation::identity();
        let buf = dev.create_uniform_buffer(&[self::Uniforms { ortho, transform }]);
        let bindings = dev.create_binding_group(&pipeline.layout.sets[0], &[&buf]);

        Self {
            pipeline,
            buf,
            bindings,
        }
    }

    fn apply(&self, pass: &mut core::Pass) {
        pass.set_pipeline(&self.pipeline);
        pass.set_binding(&self.bindings, &[]);
    }

    fn prepare(
        &'a self,
        ortho: ScreenTransformation<f32>,
    ) -> Option<(&'a core::UniformBuffer, Vec<self::Uniforms>)> {
        let transform = ScreenTransformation::identity();
        Some((&self.buf, vec![self::Uniforms { transform, ortho }]))
    }
}
