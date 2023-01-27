use std::marker::PhantomData;
use std::ops::Deref;

use bytemuck::{Pod, Zeroable};
use easygpu::prelude::*;
use easygpu::wgpu::TextureFormat;
use figures::Vectorlike;

use super::{Normal, Srgb};
use crate::math::{Angle, Pixels, Point};

/// A pipeline for rendering shapes.
pub struct Pipeline<T> {
    core: PipelineCore,
    _phantom: PhantomData<T>,
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
    pub alpha: f32,
}

impl Vertex {
    pub fn rotate_by(mut self, angle: Option<Angle>, origin: Point<f32, Pixels>) -> Self {
        if let Some(angle) = angle {
            let origin = origin.to_vector();
            let position = Point::new(self.position[0], self.position[1]);
            let relative_position = position - origin;
            let rotated = angle.transform_point(relative_position) + origin;

            self.position[0] = rotated.x;
            self.position[1] = rotated.y;
        }

        self
    }
}

impl<T> Pipeline<T> {
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

impl<'a, T> AbstractPipeline<'a> for Pipeline<T>
where
    T: VertexShaderSource,
{
    type PrepareContext = ScreenTransformation<f32>;
    type Uniforms = self::Uniforms;

    fn description() -> PipelineDescription<'a> {
        PipelineDescription {
            vertex_layout: &[
                VertexFormat::Float3,
                VertexFormat::Float2,
                VertexFormat::UByte4,
                VertexFormat::Float,
            ],
            pipeline_layout: &[
                Set(&[Binding {
                    binding: BindingType::UniformBuffer,
                    stage: ShaderStages::VERTEX,
                }]),
                Set(&[
                    Binding {
                        binding: BindingType::SampledTexture {
                            multisampled: false,
                        },
                        stage: ShaderStages::FRAGMENT,
                    },
                    Binding {
                        binding: BindingType::Sampler,
                        stage: ShaderStages::FRAGMENT,
                    },
                ]),
            ],
            vertex_shader: T::shader(),
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
                bindings,
                uniforms,
            },
            _phantom: PhantomData::default(),
        }
    }

    fn prepare(
        &'a self,
        ortho: ScreenTransformation<f32>,
    ) -> Option<(&'a UniformBuffer, Vec<self::Uniforms>)> {
        let ortho = ortho.to_array();
        let transform = ScreenTransformation::identity().to_array();
        Some((&self.uniforms, vec![self::Uniforms { ortho, transform }]))
    }
}

impl<T> Deref for Pipeline<T> {
    type Target = PipelineCore;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

/// Defines a shader source for sprites.
pub trait VertexShaderSource {
    /// The corresponding shader source type in `easygpu_lyon` for shape
    /// rendering.
    type Lyon: easygpu_lyon::VertexShaderSource + Send + Sync;

    /// The shader executable.
    #[must_use]
    fn shader() -> &'static [u8];

    /// The texture format expected.
    #[must_use]
    fn texture_format() -> TextureFormat;

    /// The sampler format expected.
    #[must_use]
    fn sampler_format() -> TextureFormat {
        <Self::Lyon as easygpu_lyon::VertexShaderSource>::sampler_format()
    }
}

impl VertexShaderSource for Srgb {
    type Lyon = easygpu_lyon::Srgb;

    fn shader() -> &'static [u8] {
        include_bytes!("shaders/sprite-srgb.vert.spv")
    }

    fn texture_format() -> TextureFormat {
        TextureFormat::Rgba8UnormSrgb
    }
}

impl VertexShaderSource for Normal {
    type Lyon = easygpu_lyon::Normal;

    fn shader() -> &'static [u8] {
        include_bytes!("shaders/sprite.vert.spv")
    }

    fn texture_format() -> TextureFormat {
        TextureFormat::Rgba8Unorm
    }
}
