use std::mem::size_of;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use figures::{Dips, Pixels, Point, Ratio, Size, UPixels, Zero};

use crate::buffer::Buffer;
use crate::{sealed, Color, RenderingGraphics};

#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(C)]
pub(crate) struct Uniforms {
    ortho: [f32; 16],
    scale: u32,
    _padding: [u32; 3],
}

impl Uniforms {
    pub fn new(size: Size<UPixels>, scale: f32) -> Self {
        let scale = Ratio::from_f32(scale);
        let scale = u32::from(scale.div_by) << 16 | u32::from(scale.mul_by);
        Self {
            ortho: ScreenTransformation::ortho(
                0.,
                0.,
                size.width.into(),
                size.height.into(),
                -1.0,
                1.0,
            )
            .into_array(),
            scale,
            _padding: [0; 3],
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct Vertex<Unit> {
    pub location: Point<Unit>,
    pub texture: Point<UPixels>,
    pub color: Color,
}

#[test]
fn vertex_align() {
    assert_eq!(std::mem::size_of::<Vertex<Dips>>(), 20);
}

pub(crate) const FLAG_DIPS: u32 = 1 << 0;
pub(crate) const FLAG_SCALE: u32 = 1 << 1;
pub(crate) const FLAG_ROTATE: u32 = 1 << 2;
pub(crate) const FLAG_TRANSLATE: u32 = 1 << 3;
pub(crate) const FLAG_TEXTURED: u32 = 1 << 4;
pub(crate) const FLAG_MASKED: u32 = 1 << 5;

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct PushConstants {
    pub flags: u32,
    pub scale: f32,
    pub rotation: f32,
    pub translation: Point<i32>,
}

#[derive(Debug)]
pub struct PreparedGraphic<Unit> {
    pub(crate) texture_binding: Option<(Arc<wgpu::BindGroup>, bool)>,
    pub(crate) vertices: Buffer<Vertex<Unit>>,
    pub(crate) indices: Buffer<u16>,
}

impl<Unit> PreparedGraphic<Unit>
where
    Unit: Default + Into<i32> + ShaderScalable + Zero,
    Vertex<Unit>: Pod,
{
    pub fn render<'pass>(
        &'pass self,
        origin: Point<Unit>,
        scale: Option<f32>,
        rotation: Option<f32>,
        graphics: &mut RenderingGraphics<'_, 'pass>,
    ) {
        graphics.active_pipeline_if_needed();

        graphics.pass.set_bind_group(
            0,
            self.texture_binding
                .as_ref()
                .map_or(&graphics.state.default_bindings, |(g, _)| g.as_ref()),
            &[],
        );

        graphics.pass.set_vertex_buffer(0, self.vertices.as_slice());
        graphics
            .pass
            .set_index_buffer(self.indices.as_slice(), wgpu::IndexFormat::Uint16);
        let mut flags = Unit::flags();
        if let Some((_, is_mask)) = &self.texture_binding {
            flags |= FLAG_TEXTURED;
            if *is_mask {
                flags |= FLAG_MASKED;
            }
        }
        let scale = scale.map_or(1., |scale| {
            flags |= FLAG_SCALE;
            scale
        });
        let rotation = rotation.map_or(0., |scale| {
            flags |= FLAG_ROTATE;
            scale
        });
        if !origin.is_zero() {
            flags |= FLAG_TRANSLATE;
        }

        graphics.pass.set_push_constants(
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            0,
            bytemuck::bytes_of(&PushConstants {
                flags,
                scale,
                rotation,
                translation: Point {
                    x: origin.x.into(),
                    y: origin.y.into(),
                },
            }),
        );
        graphics.pass.draw_indexed(
            0..self
                .indices
                .len()
                .try_into()
                .expect("too many drawn verticies"),
            0,
            0..1,
        );
    }
}

pub trait ShaderScalable: sealed::ShaderScalableSealed {}

impl ShaderScalable for Pixels {}

impl ShaderScalable for Dips {}

impl sealed::ShaderScalableSealed for Pixels {
    fn flags() -> u32 {
        0
    }
}

impl sealed::ShaderScalableSealed for Dips {
    fn flags() -> u32 {
        FLAG_DIPS
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScreenTransformation([f32; 16]);

impl ScreenTransformation {
    pub fn ortho(left: f32, top: f32, right: f32, bottom: f32, near: f32, far: f32) -> Self {
        let tx = -((right + left) / (right - left));
        let ty = -((top + bottom) / (top - bottom));
        let tz = -((far + near) / (far - near));

        // I never thought I'd write this as real code
        Self([
            // Row one
            2. / (right - left),
            0.,
            0.,
            0.,
            // Row two
            0.,
            2. / (top - bottom),
            0.,
            0.,
            // Row three
            0.,
            0.,
            -2. / (far - near),
            0.,
            // Row four
            tx,
            ty,
            tz,
            1.,
        ])
    }
}

impl ScreenTransformation {
    pub fn into_array(self) -> [f32; 16] {
        self.0
    }
}

pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

pub fn layout(
    device: &wgpu::Device,
    binding_layout: &wgpu::BindGroupLayout,
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[binding_layout],
        push_constant_ranges: &[wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            range: 0..size_of::<PushConstants>()
                .try_into()
                .expect("should fit :)"),
        }],
    })
}

pub(crate) fn bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniforms: &wgpu::Buffer,
    texture: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: uniforms,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(texture),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

pub fn new(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vertex",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: size_of::<Vertex<Dips>>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Sint32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Uint32x2,
                        offset: 8,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Uint32,
                        offset: 16,
                        shader_location: 2,
                    },
                ],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: "fragment",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),

                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}
