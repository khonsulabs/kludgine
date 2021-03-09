use crate::math::{Angle, Point, Raw};
use bytemuck::{Pod, Zeroable};
use easygpu::prelude::*;
use std::ops::Deref;

/// A pipeline for rendering shapes.
pub struct Pipeline {
    core: PipelineCore,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
/// The uniforms for the shader.
pub struct Uniforms {
    /// The orthographic projection matrix
    pub ortho: [f32; 16],
    /// The transformation matrix
    pub transform: [f32; 16],
}
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: Rgba8,
}

impl Vertex {
    pub fn rotate_by(mut self, angle: Option<Angle>, origin: Point<f32, Raw>) -> Self {
        if let Some(angle) = angle {
            let origin = origin.to_vector();
            let rotation2d = euclid::Rotation2D::new(angle);
            let position = Point::new(self.position[0], self.position[1]);
            let relative_position = position - origin;
            let rotated = rotation2d.transform_point(relative_position) + origin;

            self.position[0] = rotated.x;
            self.position[1] = rotated.y;

            self
        } else {
            self
        }
    }
}

impl Pipeline {
    pub fn binding(
        &self,
        renderer: &Renderer,
        texture: &Texture,
        sampler: &Sampler,
    ) -> BindingGroup {
        renderer
            .device
            .create_binding_group(&self.pipeline.layout.sets[1], &[texture, sampler])
    }
}

impl<'a> AbstractPipeline<'a> for Pipeline {
    type PrepareContext = ScreenTransformation<f32>;
    type Uniforms = self::Uniforms;

    fn description() -> PipelineDescription<'a> {
        PipelineDescription {
            vertex_layout: &[
                VertexFormat::Float3,
                VertexFormat::Float2,
                VertexFormat::UByte4,
            ],
            pipeline_layout: &[
                Set(&[Binding {
                    binding: BindingType::UniformBuffer,
                    stage: ShaderStage::VERTEX,
                }]),
                Set(&[
                    Binding {
                        binding: BindingType::SampledTexture,
                        stage: ShaderStage::FRAGMENT,
                    },
                    Binding {
                        binding: BindingType::Sampler,
                        stage: ShaderStage::FRAGMENT,
                    },
                ]),
            ],
            vertex_shader: include_bytes!("shaders/sprite.vert.spv"),
            fragment_shader: include_bytes!("shaders/sprite.frag.spv"),
        }
    }

    fn setup(pipeline: easygpu::pipeline::Pipeline, dev: &Device) -> Self {
        let transform = ScreenTransformation::identity().to_array();
        let ortho = ScreenTransformation::identity().to_array();
        let uniforms = dev.create_uniform_buffer(&[self::Uniforms { ortho, transform }]);
        let bindings = dev.create_binding_group(&pipeline.layout.sets[0], &[&uniforms]);

        Self {
            core: PipelineCore {
                pipeline,
                uniforms,
                bindings,
            },
        }
    }

    fn prepare(
        &'a self,
        ortho: ScreenTransformation<f32>,
    ) -> Option<(&'a UniformBuffer, Vec<self::Uniforms>)> {
        let ortho = ortho.to_array();
        let transform = ScreenTransformation::identity().to_array();
        Some((&self.uniforms, vec![self::Uniforms { transform, ortho }]))
    }
}

impl Deref for Pipeline {
    type Target = PipelineCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}
