use crate::math::{Angle, Point, Raw};
use easygpu::prelude::*;
use euclid::{Vector2D, Vector3D};
use std::ops::Deref;

/// A pipeline for rendering shapes.
pub struct Pipeline {
    core: PipelineCore,
}

#[repr(C)]
#[derive(Copy, Clone)]
/// The uniforms for the shader.
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
        let transform = ScreenTransformation::identity();
        let ortho = ScreenTransformation::identity();
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
        let transform = ScreenTransformation::identity();
        Some((&self.uniforms, vec![self::Uniforms { transform, ortho }]))
    }
}

impl Deref for Pipeline {
    type Target = PipelineCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}
