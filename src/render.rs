use std::collections::hash_map;
use std::ops::Range;
use std::sync::Arc;

use ahash::AHashMap;
use figures::traits::Zero;
use figures::Point;

use crate::buffer::Buffer;
use crate::pipeline::{
    PushConstants, ShaderScalable, Vertex, FLAG_MASKED, FLAG_ROTATE, FLAG_SCALE, FLAG_TEXTURED,
    FLAG_TRANSLATE,
};
use crate::shapes::Shape;
use crate::{sealed, Graphics, RenderingGraphics, Texture, TextureSource, VertexCollection};

pub struct Renderer<'render, 'gfx> {
    pub(crate) graphics: &'render mut Graphics<'gfx>,
    data: &'render mut Rendering,
}

#[derive(Debug)]
struct Command {
    indices: Range<u32>,
    constants: PushConstants,
    texture: Option<sealed::TextureId>,
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
            let index = self.data.vertices.get_or_insert(vertex);
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
            if texture.is_mask() {
                flags |= FLAG_MASKED;
            }
            let id = texture.id();
            if let hash_map::Entry::Vacant(entry) = self.data.textures.entry(id) {
                entry.insert(texture.bind_group());
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
                    &self.data.vertices.vertices,
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
    vertices: VertexCollection<i32>,
    indices: Vec<u16>,
    textures: AHashMap<sealed::TextureId, Arc<wgpu::BindGroup>>,
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
        self.vertices.vertex_index_by_id.clear();
        self.vertices.vertices.clear();
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
                        .set_bind_group(0, &graphics.kludgine.default_bindings, &[]);
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
