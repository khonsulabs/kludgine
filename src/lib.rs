use std::borrow::Cow;
use std::collections::{hash_map, HashMap};
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::mem::size_of;
use std::ops::{Add, Div, Neg, Range};
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::buffer::Buffer;
use crate::math::{
    Dips, Pixels, Point, Ratio, Rect, ScreenTransformation, Size, ToFloat, UPixels, Zero,
};
use crate::shapes::{
    PushConstants, Vertex, FLAG_ROTATE, FLAG_SCALE, FLAG_TEXTURED, FLAG_TRANSLATE,
};

#[cfg(feature = "app")]
pub mod app;
mod atlas;
mod buffer;
pub mod math;
mod pack;
mod sealed;
mod shapes;

pub use shapes::{Path, PathBuilder, PreparedGraphic, ShaderScalable, Shape};

pub use atlas::{CollectedTexture, TextureCollection};

pub struct Kludgine {
    default_bindings: wgpu::BindGroup,
    shapes_pipeline: wgpu::RenderPipeline,
    _shapes_shader: wgpu::ShaderModule,
    binding_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniforms: Buffer<Uniforms>,
    fonts: cosmic_text::FontSystem,
    cache: cosmic_text::SwashCache,
    text_atlas: TextureCollection,
}

impl Kludgine {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        initial_size: Size<Pixels>,
        scale: f32,
    ) -> Self {
        let uniforms = Buffer::new(
            &[Uniforms {
                ortho: ScreenTransformation::ortho(
                    0.,
                    0.,
                    initial_size.width.into(),
                    initial_size.height.into(),
                    -1.0,
                    1.0,
                )
                .into_array(),
                scale: Ratio::from_f32(scale),
                _padding: [0; 3],
            }],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            device,
        );

        let binding_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&binding_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                range: 0..size_of::<PushConstants>() as u32,
            }],
        });

        let empty_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let shapes_bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &binding_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &uniforms.wgpu,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &empty_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let shapes_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shapes.wgsl"))),
        });

        let shapes_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shapes"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shapes_shader,
                entry_point: "vs_main",
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
                module: &shapes_shader,
                entry_point: "fs_main",
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
        });

        let fonts = cosmic_text::FontSystem::new();

        Self {
            text_atlas: TextureCollection::new(
                Size::new(512, 512),
                128,
                wgpu::TextureFormat::Bgra8Unorm,
                &ProtoGraphics {
                    device,
                    queue,
                    binding_layout: &binding_layout,
                    sampler: &sampler,
                    uniforms: &uniforms.wgpu,
                },
            ),

            default_bindings: shapes_bindings,
            shapes_pipeline,
            _shapes_shader: shapes_shader,
            sampler,

            uniforms,
            binding_layout,

            fonts,
            cache: cosmic_text::SwashCache::new(),
        }
    }

    pub fn resize(&self, new_size: Size<Pixels>, new_scale: f32, queue: &wgpu::Queue) {
        self.uniforms.update(
            0,
            &[Uniforms {
                ortho: ScreenTransformation::ortho(
                    0.,
                    0.,
                    new_size.width.into(),
                    new_size.height.into(),
                    -1.0,
                    1.0,
                )
                .into_array(),
                scale: Ratio::from_f32(new_scale),
                _padding: [0; 3],
            }],
            queue,
        );
    }
}

pub trait WgpuDeviceAndQueue {
    fn device(&self) -> &wgpu::Device;
    fn queue(&self) -> &wgpu::Queue;
    fn binding_layout(&self) -> &wgpu::BindGroupLayout;
    fn uniforms(&self) -> &wgpu::Buffer;
    fn sampler(&self) -> &wgpu::Sampler;
}

struct ProtoGraphics<'gfx> {
    device: &'gfx wgpu::Device,
    queue: &'gfx wgpu::Queue,
    binding_layout: &'gfx wgpu::BindGroupLayout,
    sampler: &'gfx wgpu::Sampler,
    uniforms: &'gfx wgpu::Buffer,
}

impl WgpuDeviceAndQueue for ProtoGraphics<'_> {
    fn device(&self) -> &wgpu::Device {
        self.device
    }

    fn queue(&self) -> &wgpu::Queue {
        self.queue
    }

    fn binding_layout(&self) -> &wgpu::BindGroupLayout {
        self.binding_layout
    }

    fn uniforms(&self) -> &wgpu::Buffer {
        self.uniforms
    }

    fn sampler(&self) -> &wgpu::Sampler {
        self.sampler
    }
}

impl WgpuDeviceAndQueue for Graphics<'_> {
    fn device(&self) -> &wgpu::Device {
        self.device
    }

    fn queue(&self) -> &wgpu::Queue {
        self.queue
    }

    fn binding_layout(&self) -> &wgpu::BindGroupLayout {
        &self.kludgine.binding_layout
    }

    fn uniforms(&self) -> &wgpu::Buffer {
        &self.kludgine.uniforms.wgpu
    }

    fn sampler(&self) -> &wgpu::Sampler {
        &self.kludgine.sampler
    }
}

pub struct Graphics<'gfx> {
    kludgine: &'gfx mut Kludgine,
    device: &'gfx wgpu::Device,
    queue: &'gfx wgpu::Queue, // Need this eventually to be able to have dynamic shape collections
}

impl<'gfx> Graphics<'gfx> {
    pub const fn device(&self) -> &'gfx wgpu::Device {
        self.device
    }

    pub const fn queue(&self) -> &'gfx wgpu::Queue {
        self.queue
    }

    pub fn font_system(&mut self) -> &mut cosmic_text::FontSystem {
        &mut self.kludgine.fonts
    }
}

impl<'gfx> Graphics<'gfx> {
    pub fn new(
        kludgine: &'gfx mut Kludgine,
        device: &'gfx wgpu::Device,
        queue: &'gfx wgpu::Queue,
    ) -> Self {
        Self {
            kludgine,
            device,
            queue,
        }
    }
}

pub struct RenderingGraphics<'gfx, 'pass> {
    pass: &'gfx mut wgpu::RenderPass<'pass>,
    state: &'pass Kludgine,
    device: &'gfx wgpu::Device,
    queue: &'gfx wgpu::Queue,
    pipeline_is_active: bool,
}

impl<'gfx, 'pass> RenderingGraphics<'gfx, 'pass> {
    pub fn new(
        pass: &'gfx mut wgpu::RenderPass<'pass>,
        state: &'pass Kludgine,
        device: &'gfx wgpu::Device,
        queue: &'gfx wgpu::Queue,
    ) -> Self {
        Self {
            pass,
            state,
            device,
            queue,
            pipeline_is_active: false,
        }
    }
    pub const fn device(&self) -> &'gfx wgpu::Device {
        self.device
    }

    pub const fn queue(&self) -> &'gfx wgpu::Queue {
        self.queue
    }

    pub fn render_pass(&mut self) -> &mut wgpu::RenderPass<'pass> {
        // When we expose the render pass, we can't guarantee we're the current pipeline anymore.
        self.pipeline_is_active = false;
        self.pass
    }

    fn active_pipeline_if_needed(&mut self) -> bool {
        if self.pipeline_is_active {
            false
        } else {
            self.pipeline_is_active = true;
            self.pass.set_pipeline(&self.state.shapes_pipeline);
            true
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct Color(u32);

impl Color {
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self((red as u32) << 24 | (green as u32) << 16 | (blue as u32) << 8 | alpha as u32)
    }

    pub fn new_f32(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self::new(
            (red * 255.).round() as u8,
            (green * 255.).round() as u8,
            (blue * 255.).round() as u8,
            (alpha * 255.).round() as u8,
        )
    }

    pub const fn red(&self) -> u8 {
        (self.0 >> 24) as u8
    }

    pub fn red_f32(&self) -> f32 {
        self.red() as f32 / 255.
    }

    pub const fn green(&self) -> u8 {
        (self.0 >> 16) as u8
    }

    pub fn green_f32(&self) -> f32 {
        self.green() as f32 / 255.
    }

    pub const fn blue(&self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub fn blue_f32(&self) -> f32 {
        self.blue() as f32 / 255.
    }

    pub const fn alpha(&self) -> u8 {
        self.0 as u8
    }

    pub fn alpha_f32(&self) -> f32 {
        self.alpha() as f32 / 255.
    }
}

impl Debug for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "#{:08x}", self.0)
    }
}

impl From<Color> for wgpu::Color {
    fn from(color: Color) -> Self {
        Self {
            r: color.red_f32() as f64,
            g: color.green_f32() as f64,
            b: color.blue_f32() as f64,
            a: color.alpha_f32() as f64,
        }
    }
}

#[test]
fn color_debug() {
    assert_eq!(format!("{:?}", Color::new(1, 2, 3, 4)), "#01020304");
}

impl Color {
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ALICEBLUE: Self = Self::new(240, 248, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ANTIQUEWHITE: Self = Self::new(250, 235, 215, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const AQUA: Self = Self::new(0, 255, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const AQUAMARINE: Self = Self::new(127, 255, 212, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const AZURE: Self = Self::new(240, 255, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BEIGE: Self = Self::new(245, 245, 220, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BISQUE: Self = Self::new(255, 228, 196, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BLACK: Self = Self::new(0, 0, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BLANCHEDALMOND: Self = Self::new(255, 235, 205, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BLUE: Self = Self::new(0, 0, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BLUEVIOLET: Self = Self::new(138, 43, 226, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BROWN: Self = Self::new(165, 42, 42, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BURLYWOOD: Self = Self::new(222, 184, 135, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CADETBLUE: Self = Self::new(95, 158, 160, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CHARTREUSE: Self = Self::new(127, 255, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CHOCOLATE: Self = Self::new(210, 105, 30, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CLEAR_BLACK: Self = Self::new(0, 0, 0, 0);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CLEAR_WHITE: Self = Self::new(255, 255, 255, 0);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CORAL: Self = Self::new(255, 127, 80, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CORNFLOWERBLUE: Self = Self::new(100, 149, 237, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CORNSILK: Self = Self::new(255, 248, 220, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CRIMSON: Self = Self::new(220, 20, 60, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CYAN: Self = Self::new(0, 255, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKBLUE: Self = Self::new(0, 0, 139, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKCYAN: Self = Self::new(0, 139, 139, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKGOLDENROD: Self = Self::new(184, 134, 11, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKGRAY: Self = Self::new(169, 169, 169, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKGREEN: Self = Self::new(0, 100, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKGREY: Self = Self::new(169, 169, 169, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKKHAKI: Self = Self::new(189, 183, 107, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKMAGENTA: Self = Self::new(139, 0, 139, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKOLIVEGREEN: Self = Self::new(85, 107, 47, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKORANGE: Self = Self::new(255, 140, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKORCHID: Self = Self::new(153, 50, 204, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKRED: Self = Self::new(139, 0, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKSALMON: Self = Self::new(233, 150, 122, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKSEAGREEN: Self = Self::new(143, 188, 143, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKSLATEBLUE: Self = Self::new(72, 61, 139, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKSLATEGRAY: Self = Self::new(47, 79, 79, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKSLATEGREY: Self = Self::new(47, 79, 79, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKTURQUOISE: Self = Self::new(0, 206, 209, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKVIOLET: Self = Self::new(148, 0, 211, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DEEPPINK: Self = Self::new(255, 20, 147, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DEEPSKYBLUE: Self = Self::new(0, 191, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DIMGRAY: Self = Self::new(105, 105, 105, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DIMGREY: Self = Self::new(105, 105, 105, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DODGERBLUE: Self = Self::new(30, 144, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const FIREBRICK: Self = Self::new(178, 34, 34, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const FLORALWHITE: Self = Self::new(255, 250, 240, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const FORESTGREEN: Self = Self::new(34, 139, 34, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const FUCHSIA: Self = Self::new(255, 0, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GAINSBORO: Self = Self::new(220, 220, 220, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GHOSTWHITE: Self = Self::new(248, 248, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GOLD: Self = Self::new(255, 215, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GOLDENROD: Self = Self::new(218, 165, 32, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GRAY: Self = Self::new(128, 128, 128, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GREEN: Self = Self::new(0, 128, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GREENYELLOW: Self = Self::new(173, 255, 47, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GREY: Self = Self::new(128, 128, 128, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const HONEYDEW: Self = Self::new(240, 255, 240, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const HOTPINK: Self = Self::new(255, 105, 180, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const INDIANRED: Self = Self::new(205, 92, 92, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const INDIGO: Self = Self::new(75, 0, 130, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const IVORY: Self = Self::new(255, 255, 240, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const KHAKI: Self = Self::new(240, 230, 140, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LAVENDER: Self = Self::new(230, 230, 250, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LAVENDERBLUSH: Self = Self::new(255, 240, 245, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LAWNGREEN: Self = Self::new(124, 252, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LEMONCHIFFON: Self = Self::new(255, 250, 205, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTBLUE: Self = Self::new(173, 216, 230, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTCORAL: Self = Self::new(240, 128, 128, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTCYAN: Self = Self::new(224, 255, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTGOLDENRODYELLOW: Self = Self::new(250, 250, 210, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTGRAY: Self = Self::new(211, 211, 211, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTGREEN: Self = Self::new(144, 238, 144, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTGREY: Self = Self::new(211, 211, 211, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTPINK: Self = Self::new(255, 182, 193, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTSALMON: Self = Self::new(255, 160, 122, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTSEAGREEN: Self = Self::new(32, 178, 170, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTSKYBLUE: Self = Self::new(135, 206, 250, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTSLATEGRAY: Self = Self::new(119, 136, 153, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTSLATEGREY: Self = Self::new(119, 136, 153, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTSTEELBLUE: Self = Self::new(176, 196, 222, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTYELLOW: Self = Self::new(255, 255, 224, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIME: Self = Self::new(0, 255, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIMEGREEN: Self = Self::new(50, 205, 50, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LINEN: Self = Self::new(250, 240, 230, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MAGENTA: Self = Self::new(255, 0, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MAROON: Self = Self::new(128, 0, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMAQUAMARINE: Self = Self::new(102, 205, 170, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMBLUE: Self = Self::new(0, 0, 205, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMORCHID: Self = Self::new(186, 85, 211, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMPURPLE: Self = Self::new(147, 112, 219, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMSEAGREEN: Self = Self::new(60, 179, 113, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMSLATEBLUE: Self = Self::new(123, 104, 238, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMSPRINGGREEN: Self = Self::new(0, 250, 154, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMTURQUOISE: Self = Self::new(72, 209, 204, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMVIOLETRED: Self = Self::new(199, 21, 133, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MIDNIGHTBLUE: Self = Self::new(25, 25, 112, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MINTCREAM: Self = Self::new(245, 255, 250, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MISTYROSE: Self = Self::new(255, 228, 225, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MOCCASIN: Self = Self::new(255, 228, 181, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const NAVAJOWHITE: Self = Self::new(255, 222, 173, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const NAVY: Self = Self::new(0, 0, 128, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const OLDLACE: Self = Self::new(253, 245, 230, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const OLIVE: Self = Self::new(128, 128, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const OLIVEDRAB: Self = Self::new(107, 142, 35, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ORANGE: Self = Self::new(255, 165, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ORANGERED: Self = Self::new(255, 69, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ORCHID: Self = Self::new(218, 112, 214, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PALEGOLDENROD: Self = Self::new(238, 232, 170, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PALEGREEN: Self = Self::new(152, 251, 152, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PALETURQUOISE: Self = Self::new(175, 238, 238, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PALEVIOLETRED: Self = Self::new(219, 112, 147, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PAPAYAWHIP: Self = Self::new(255, 239, 213, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PEACHPUFF: Self = Self::new(255, 218, 185, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PERU: Self = Self::new(205, 133, 63, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PINK: Self = Self::new(255, 192, 203, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PLUM: Self = Self::new(221, 160, 221, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const POWDERBLUE: Self = Self::new(176, 224, 230, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PURPLE: Self = Self::new(128, 0, 128, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const REBECCAPURPLE: Self = Self::new(102, 51, 153, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const RED: Self = Self::new(255, 0, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ROSYBROWN: Self = Self::new(188, 143, 143, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ROYALBLUE: Self = Self::new(65, 105, 225, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SADDLEBROWN: Self = Self::new(139, 69, 19, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SALMON: Self = Self::new(250, 128, 114, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SANDYBROWN: Self = Self::new(244, 164, 96, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SEAGREEN: Self = Self::new(46, 139, 87, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SEASHELL: Self = Self::new(255, 245, 238, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SIENNA: Self = Self::new(160, 82, 45, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SILVER: Self = Self::new(192, 192, 192, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SKYBLUE: Self = Self::new(135, 206, 235, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SLATEBLUE: Self = Self::new(106, 90, 205, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SLATEGRAY: Self = Self::new(112, 128, 144, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SLATEGREY: Self = Self::new(112, 128, 144, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SNOW: Self = Self::new(255, 250, 250, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SPRINGGREEN: Self = Self::new(0, 255, 127, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const STEELBLUE: Self = Self::new(70, 130, 180, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const TAN: Self = Self::new(210, 180, 140, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const TEAL: Self = Self::new(0, 128, 128, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const THISTLE: Self = Self::new(216, 191, 216, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const TOMATO: Self = Self::new(255, 99, 71, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const TURQUOISE: Self = Self::new(64, 224, 208, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const VIOLET: Self = Self::new(238, 130, 238, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const WHEAT: Self = Self::new(245, 222, 179, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const WHITE: Self = Self::new(255, 255, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const WHITESMOKE: Self = Self::new(245, 245, 245, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const YELLOW: Self = Self::new(255, 255, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const YELLOWGREEN: Self = Self::new(154, 205, 50, 255);
}

#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(C)]
struct Uniforms {
    ortho: [f32; 16],
    scale: Ratio,
    _padding: [u32; 3],
}

#[derive(Debug)]
pub struct Texture {
    id: sealed::TextureId,
    wgpu: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: Arc<wgpu::BindGroup>,
}

impl Texture {
    pub fn new(
        graphics: &impl WgpuDeviceAndQueue,
        size: Size<UPixels>,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        let wgpu = graphics.device().create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: size.into(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        let view = wgpu.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = Arc::new(create_bind_group(graphics, &view));
        Self {
            id: sealed::TextureId::new_unique_id(),
            wgpu,
            view,
            bind_group,
        }
    }

    pub fn new_with_data(
        graphics: &impl WgpuDeviceAndQueue,
        size: Size<UPixels>,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        data: &[u8],
    ) -> Self {
        let wgpu = graphics.device().create_texture_with_data(
            graphics.queue(),
            &wgpu::TextureDescriptor {
                label: None,
                size: size.into(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            },
            data,
        );
        let view = wgpu.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = Arc::new(create_bind_group(graphics, &view));
        Self {
            id: sealed::TextureId::new_unique_id(),
            wgpu,
            view,
            bind_group,
        }
    }

    pub fn create_render_pass<'gfx>(
        &'gfx self,
        encoder: &'gfx mut wgpu::CommandEncoder,
        load_op: wgpu::LoadOp<Color>,
    ) -> wgpu::RenderPass<'gfx> {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: match load_op {
                        wgpu::LoadOp::Clear(color) => wgpu::LoadOp::Clear(color.into()),
                        wgpu::LoadOp::Load => wgpu::LoadOp::Load,
                    },
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });
        pass
    }

    #[cfg(feature = "image")]
    pub fn from_image(image: &image::DynamicImage, graphics: &Graphics<'_>) -> Self {
        let image = image.to_rgba8();
        Self::new_with_data(
            graphics,
            Size::new(image.width(), image.height()),
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::TEXTURE_BINDING,
            image.as_raw(),
        )
    }

    pub fn prepare_sized<Unit>(
        &self,
        size: Size<Unit>,
        graphics: &Graphics<'_>,
    ) -> PreparedGraphic<Unit>
    where
        Unit: Add<Output = Unit>
            + ToFloat<Float = f32>
            + Div<i32, Output = Unit>
            + Neg<Output = Unit>
            + Ord
            + From<i32>
            + Copy
            + Debug
            + Default,
        Vertex<Unit>: bytemuck::Pod,
    {
        self.prepare(Rect::new(Point::default(), size), graphics)
    }

    pub fn prepare<Unit>(&self, dest: Rect<Unit>, graphics: &Graphics<'_>) -> PreparedGraphic<Unit>
    where
        Unit: Add<Output = Unit>
            + ToFloat<Float = f32>
            + Div<i32, Output = Unit>
            + Neg<Output = Unit>
            + From<i32>
            + Ord
            + Copy
            + Debug,
        Vertex<Unit>: bytemuck::Pod,
    {
        self.prepare_partial(self.size().into(), dest, graphics)
    }

    pub fn prepare_partial<Unit>(
        &self,
        source: Rect<UPixels>,
        dest: Rect<Unit>,
        graphics: &Graphics<'_>,
    ) -> PreparedGraphic<Unit>
    where
        Unit: Add<Output = Unit>
            + ToFloat<Float = f32>
            + Div<i32, Output = Unit>
            + Neg<Output = Unit>
            + From<i32>
            + Ord
            + Copy
            + Debug,
        Vertex<Unit>: bytemuck::Pod,
    {
        let (source_top_left, source_bottom_right) = source.extents();
        let (dest_top_left, dest_bottom_right) = dest.extents();
        let path = PathBuilder::new_textured(dest_top_left, source_top_left)
            .line_to(
                Point::new(dest_bottom_right.x, dest_top_left.y),
                Point::new(source_bottom_right.x, source_top_left.y),
            )
            .line_to(dest_bottom_right, source_bottom_right)
            .line_to(
                Point::new(dest_top_left.x, dest_bottom_right.y),
                Point::new(source_top_left.x, source_bottom_right.y),
            )
            .close();
        path.fill(Color::new(255, 255, 255, 255))
            .prepare(self, graphics)
    }

    // pub fn read_into(&self, destination: &mut Vec<u8>, device: &wgpu::) {
    //     dev
    // }
    pub fn size(&self) -> Size<UPixels> {
        Size {
            width: UPixels(self.wgpu.width()),
            height: UPixels(self.wgpu.height()),
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.wgpu.format()
    }
}

fn create_bind_group(
    graphics: &impl WgpuDeviceAndQueue,
    view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    graphics
        .device()
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: graphics.binding_layout(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: graphics.uniforms(),
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(graphics.sampler()),
                },
            ],
        })
}

// pub struct PreparedTexture<Unit> {
//     shape: PreparedShape<Unit>,
// }

// impl<Unit> PreparedTexture<Unit> {
//     pub fn render<'pass>(
//         &'pass self,
//         origin: Point<Unit>,
//         scale: Option<f32>,
//         rotation: Option<f32>,
//         graphics: &mut RenderingGraphics<'_, 'pass>,
//     ) where
//         Unit: Default + Into<i32> + ShaderScalable + Zero,
//         Vertex<Unit>: Pod,
//     {
//         if graphics.active_bindings.is_none() {
//             graphics.active_bindings = Some(Pipeline::Texture);
//             graphics.pass.set_pipeline(&graphics.state.shapes_pipeline);
//         }
//         graphics.pass.set_bind_group(0, &self.bind_group, &[]);
//         self.shape.render_direct(origin, scale, rotation, graphics);
//     }
// }

pub trait TextureSource: sealed::TextureSource {}

impl TextureSource for Texture {}

impl sealed::TextureSource for Texture {
    fn bind_group(&self, _graphics: &Graphics<'_>) -> Arc<wgpu::BindGroup> {
        self.bind_group.clone()
    }

    fn id(&self) -> sealed::TextureId {
        self.id
    }
}

pub struct Renderer<'render, 'gfx> {
    graphics: &'render mut Graphics<'gfx>,
    data: &'render mut Rendering,
}

#[derive(Debug)]
struct Command {
    indices: Range<u32>,
    constants: PushConstants,
    texture: Option<sealed::TextureId>,
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
struct VertexId(Vertex<i32>);

impl Hash for VertexId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        bytemuck::bytes_of(&self.0).hash(state);
    }
}

impl Renderer<'_, '_> {
    pub fn draw_shape<Unit>(
        &mut self,
        shape: &Shape<Unit, false>,
        origin: Point<Unit>,
        rotation_rads: Option<f32>,
        scale: Option<f32>,
    ) where
        Unit: Into<i32> + Zero + Copy,
        Unit: ShaderScalable,
    {
        self.inner_draw(
            shape,
            Option::<&Texture>::None,
            origin,
            rotation_rads,
            scale,
        );
    }

    pub fn draw_textured_shape<Unit>(
        &mut self,
        shape: &Shape<Unit, true>,
        texture: &impl TextureSource,
        origin: Point<Unit>,
        rotation_rads: Option<f32>,
        scale: Option<f32>,
    ) where
        Unit: Into<i32> + Zero + Copy,
        Unit: ShaderScalable,
    {
        self.inner_draw(shape, Some(texture), origin, rotation_rads, scale);
    }

    fn inner_draw<Unit, const TEXTURED: bool>(
        &mut self,
        shape: &Shape<Unit, TEXTURED>,
        texture: Option<&impl TextureSource>,
        origin: Point<Unit>,
        rotation_rads: Option<f32>,
        scale: Option<f32>,
    ) where
        Unit: Into<i32> + Zero + Copy,
        Unit: ShaderScalable,
    {
        // Merge the vertices into the graphics
        let mut vertex_map = Vec::with_capacity(shape.vertices.len());
        for vertex in shape.vertices.iter().copied() {
            let vertex = Vertex {
                location: Point {
                    x: vertex.location.x.into(),
                    y: vertex.location.y.into(),
                },
                texture: vertex.texture,
                color: vertex.color,
            };
            let index = *self
                .data
                .vertex_index_by_id
                .entry(VertexId(vertex))
                .or_insert_with(|| {
                    let index = self
                        .data
                        .vertices
                        .len()
                        .try_into()
                        .expect("too many vertices being drawn");
                    self.data.vertices.push(vertex);
                    index
                });
            vertex_map.push(index);
        }

        let first_index_drawn = self.data.indices.len();
        for &vertex_index in &shape.indices {
            self.data
                .indices
                .push(vertex_map[usize::from(vertex_index)]);
        }

        let mut flags = Unit::flags();
        assert_eq!(TEXTURED, texture.is_some());
        let texture = if let Some(texture) = texture {
            flags |= FLAG_TEXTURED;
            let id = texture.id();
            if let hash_map::Entry::Vacant(entry) = self.data.textures.entry(id) {
                entry.insert(texture.bind_group(self.graphics));
            }
            Some(id)
        } else {
            None
        };
        let scale = scale.map_or(1., |scale| {
            flags |= FLAG_SCALE;
            scale
        });
        let rotation = rotation_rads.map_or(0., |scale| {
            flags |= FLAG_ROTATE;
            scale
        });
        if !origin.is_zero() {
            flags |= FLAG_TRANSLATE;
        }

        self.data.commands.push(Command {
            indices: first_index_drawn
                .try_into()
                .expect("too many drawn verticies")
                ..self
                    .data
                    .indices
                    .len()
                    .try_into()
                    .expect("too many drawn verticies"),
            constants: PushConstants {
                flags,
                scale,
                rotation,
                translation: Point {
                    x: origin.x.into(),
                    y: origin.y.into(),
                },
            },
            texture,
        });
    }
}

impl Drop for Renderer<'_, '_> {
    fn drop(&mut self) {
        if self.data.indices.is_empty() {
            self.data.buffers = None;
        } else {
            self.data.buffers = Some(RenderingBuffers {
                vertex: Buffer::new(
                    &self.data.vertices,
                    wgpu::BufferUsages::VERTEX,
                    self.graphics.device,
                ),
                index: Buffer::new(
                    &self.data.indices,
                    wgpu::BufferUsages::INDEX,
                    self.graphics.device,
                ),
            });
        }
    }
}

#[derive(Default, Debug)]
pub struct Rendering {
    buffers: Option<RenderingBuffers>,
    vertices: Vec<Vertex<i32>>,
    vertex_index_by_id: HashMap<VertexId, u16>,
    indices: Vec<u16>,
    textures: HashMap<sealed::TextureId, Arc<wgpu::BindGroup>>,
    commands: Vec<Command>,
}

#[derive(Debug)]
struct RenderingBuffers {
    vertex: Buffer<Vertex<i32>>,
    index: Buffer<u16>,
}

impl Rendering {
    pub fn new_frame<'rendering, 'gfx>(
        &'rendering mut self,
        graphics: &'rendering mut Graphics<'gfx>,
    ) -> Renderer<'rendering, 'gfx> {
        self.commands.clear();
        self.indices.clear();
        self.textures.clear();
        self.vertex_index_by_id.clear();
        self.vertices.clear();
        Renderer {
            graphics,
            data: self,
        }
    }

    pub fn render<'pass>(&'pass self, graphics: &mut RenderingGraphics<'_, 'pass>) {
        if let Some(buffers) = &self.buffers {
            let mut current_texture_id = None;
            let mut needs_texture_binding = graphics.active_pipeline_if_needed();

            graphics
                .pass
                .set_vertex_buffer(0, buffers.vertex.as_slice());
            graphics
                .pass
                .set_index_buffer(buffers.index.as_slice(), wgpu::IndexFormat::Uint16);

            for command in &self.commands {
                if let Some(texture_id) = &command.texture {
                    if current_texture_id != Some(*texture_id) {
                        current_texture_id = Some(*texture_id);
                        graphics.pass.set_bind_group(
                            0,
                            self.textures.get(texture_id).expect("texture missing"),
                            &[],
                        );
                    }
                } else if needs_texture_binding {
                    needs_texture_binding = false;
                    graphics
                        .pass
                        .set_bind_group(0, &graphics.state.default_bindings, &[]);
                }

                graphics.pass.set_push_constants(
                    wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    0,
                    bytemuck::bytes_of(&command.constants),
                );
                graphics.pass.draw_indexed(command.indices.clone(), 0, 0..1);
            }
        }
    }
}
