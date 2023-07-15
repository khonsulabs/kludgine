use std::collections::hash_map;
use std::ops::{Add, Deref, DerefMut, Range};
use std::sync::Arc;

use ahash::AHashMap;
use figures::{Angle, FloatConversion, IntoSigned, IsZero, Point, Rect};
use shelf_packer::UPx;

use crate::buffer::DiffableBuffer;
use crate::pipeline::{
    PushConstants, ShaderScalable, Vertex, FLAG_MASKED, FLAG_ROTATE, FLAG_SCALE, FLAG_TEXTURED,
    FLAG_TRANSLATE,
};
use crate::shapes::Shape;
use crate::{
    sealed, ClipGuard, ClipRect, Clipped, Color, Graphics, RenderingGraphics, ShapeSource, Texture,
    TextureBlit, TextureSource, VertexCollection,
};

/// An easy-to-use graphics renderer that batches operations on the GPU
/// automatically.
///
/// Using the draw operations on this type don't immediately draw. Instead, once
/// this type is dropped, the [`Rendering`] that created this renderer will be
/// updated with the new drawing instructions. All of the pending operations can
/// be drawn using [`Rendering::render`].
#[derive(Debug)]
pub struct Renderer<'render, 'gfx> {
    pub(crate) graphics: &'render mut Graphics<'gfx>,
    data: &'render mut Drawing,
    clip_index: u32,
}

impl<'gfx> Deref for Renderer<'_, 'gfx> {
    type Target = Graphics<'gfx>;

    fn deref(&self) -> &Self::Target {
        self.graphics
    }
}

impl<'gfx> DerefMut for Renderer<'_, 'gfx> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.graphics
    }
}

#[derive(Debug)]
struct Command {
    clip_index: u32,
    indices: Range<u32>,
    constants: PushConstants,
    texture: Option<sealed::TextureId>,
}

impl<'render, 'gfx> Renderer<'render, 'gfx> {
    /// Draws a shape at the origin, rotating and scaling as needed.
    pub fn draw_shape<Unit>(
        &mut self,
        shape: &Shape<Unit, false>,
        origin: Point<Unit>,
        rotation_rads: Option<Angle>,
        scale: Option<f32>,
    ) where
        Unit: IsZero + ShaderScalable + IntoSigned + Copy,
        i32: From<<Unit as IntoSigned>::Signed>,
    {
        self.inner_draw(
            shape,
            Option::<&Texture>::None,
            origin,
            rotation_rads,
            scale,
        );
    }

    /// Draws `texture` at `destination`, scaling as necessary.
    pub fn draw_texture<Unit>(&mut self, texture: &impl TextureSource, destination: Rect<Unit>)
    where
        Unit: Default
            + FloatConversion<Float = f32>
            + Add<Output = Unit>
            + Ord
            + IsZero
            + Copy
            + IsZero
            + ShaderScalable
            + IntoSigned
            + Copy,
        i32: From<<Unit as IntoSigned>::Signed>,
    {
        self.draw_textured_shape(
            &TextureBlit::new(texture.default_rect(), destination, Color::WHITE),
            texture,
            Point::default(),
            None,
            None,
        );
    }

    /// Draws a shape that was created with texture coordinates, applying the
    /// provided texture.
    pub fn draw_textured_shape<Unit>(
        &mut self,
        shape: &impl ShapeSource<Unit, true>,
        texture: &impl TextureSource,
        origin: Point<Unit>,
        rotation: Option<Angle>,
        scale: Option<f32>,
    ) where
        Unit: IsZero + ShaderScalable + IntoSigned + Copy,
        i32: From<<Unit as IntoSigned>::Signed>,
    {
        self.inner_draw(shape, Some(texture), origin, rotation, scale);
    }

    fn inner_draw<Unit, const TEXTURED: bool>(
        &mut self,
        shape: &impl ShapeSource<Unit, TEXTURED>,
        texture: Option<&impl TextureSource>,
        origin: Point<Unit>,
        rotation: Option<Angle>,
        scale: Option<f32>,
    ) where
        Unit: IsZero + ShaderScalable + IntoSigned + Copy,
        i32: From<<Unit as IntoSigned>::Signed>,
    {
        // Merge the vertices into the graphics
        let vertices = shape.vertices();
        let mut vertex_map = Vec::with_capacity(vertices.len());
        for vertex in vertices {
            let vertex = Vertex {
                location: vertex.location.into_signed().cast(),
                texture: vertex.texture,
                color: vertex.color,
            };
            let index = self.data.vertices.get_or_insert(vertex);
            vertex_map.push(index);
        }

        let first_index_drawn = self.data.indices.len();
        for &vertex_index in shape.indices() {
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
        let rotation = rotation.map_or(0., |scale| {
            flags |= FLAG_ROTATE;
            scale.into_raidans_f()
        });
        if !origin.is_zero() {
            flags |= FLAG_TRANSLATE;
        }

        let constants = PushConstants {
            flags,
            scale,
            rotation,
            translation: origin.into_signed().cast(),
        };

        match self.data.commands.last_mut() {
            Some(Command {
                clip_index,
                texture: last_texture,
                indices,
                constants: last_constants,
            }) if clip_index == &self.clip_index
                && last_texture == &texture
                && last_constants == &constants =>
            {
                // Batch this draw operation with the previous one.
                indices.end = self
                    .data
                    .indices
                    .len()
                    .try_into()
                    .expect("too many drawn verticies");
            }
            _ => {
                self.data.commands.push(Command {
                    clip_index: self.clip_index,
                    indices: first_index_drawn
                        .try_into()
                        .expect("too many drawn verticies")
                        ..self
                            .data
                            .indices
                            .len()
                            .try_into()
                            .expect("too many drawn verticies"),
                    constants,
                    texture,
                });
            }
        }
    }

    /// Returns the number of vertexes that compose the drawing commands.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.data.vertices.vertices.len()
    }

    /// Returns the number of triangles that are being rendered in the drawing
    /// commands.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.data.indices.len() / 3
    }

    /// Returns the number of drawing operations that will be sent to the GPU
    /// during [`render()`](Rendering::render).
    #[must_use]
    pub fn command_count(&self) -> usize {
        self.data.commands.len()
    }

    /// Returns a [`ClipGuard`] that causes all drawing operations to be offset
    /// and clipped to `clip` until it is dropped.
    ///
    /// This function causes the [`Renderer`] to act as if the origin of the
    /// context is `clip.origin`, and the size of the context is `clip.size`.
    /// This means that rendering at 0,0 will actually render at the effective
    /// clip rect's origin.
    ///
    /// `clip` is relative to the current clip rect and cannot extend the
    /// current clipping rectangle.
    pub fn clipped_to(&mut self, clip: Rect<UPx>) -> ClipGuard<'_, Self> {
        let previous_clip = self.clip;
        self.clip = previous_clip.clip_to(clip);
        self.clip_index = self.data.get_or_lookup_clip(self.clip);

        ClipGuard {
            previous_clip,
            clipped: self,
        }
    }
}

impl Clipped for Renderer<'_, '_> {}

impl sealed::Clipped for Renderer<'_, '_> {
    fn restore_clip(&mut self, previous_clip: ClipRect) {
        self.graphics.restore_clip(previous_clip);
        self.clip_index = self.data.get_or_lookup_clip(previous_clip);
    }
}

#[cfg(feature = "cosmic-text")]
mod text {
    use std::array;
    use std::collections::hash_map;
    use std::sync::Arc;

    use ahash::AHashMap;
    use figures::units::Px;
    use figures::{ScreenScale, ScreenUnit};

    use super::{
        Angle, Color, Command, IntoSigned, IsZero, Point, PushConstants, Renderer, Vertex,
        FLAG_MASKED, FLAG_ROTATE, FLAG_SCALE, FLAG_TEXTURED, FLAG_TRANSLATE,
    };
    use crate::sealed::{ShaderScalableSealed, ShapeSource, TextureId, TextureSource};
    use crate::text::{map_each_glyph, measure_text, CachedGlyphHandle, MeasuredText, TextOrigin};
    use crate::{TextureBlit, VertexCollection};

    impl<'gfx> Renderer<'_, 'gfx> {
        /// Measures `text` using the current text settings.
        ///
        /// `default_color` does not affect the
        pub fn measure_text<Unit>(&mut self, text: &str, default_color: Color) -> MeasuredText<Unit>
        where
            Unit: figures::ScreenUnit,
        {
            self.update_scratch_buffer(text);
            measure_text::<Unit, true>(
                None,
                default_color,
                self.graphics.kludgine,
                self.graphics.queue,
                &mut self.data.glyphs,
            )
        }

        /// Draws `text` using the current text settings.
        pub fn draw_text<Unit>(
            &mut self,
            text: &str,
            color: Color,
            origin: TextOrigin<Unit>,
            translate: Point<Unit>,
            rotation: Option<Angle>,
            scale: Option<f32>,
        ) where
            Unit: ScreenUnit,
        {
            self.graphics.kludgine.update_scratch_buffer(text);
            self.draw_text_buffer_inner(
                None,
                color,
                origin.into_px(self.scale()),
                translate,
                rotation,
                scale,
            );
        }

        /// Prepares the text layout contained in `buffer` to be rendered.
        ///
        /// When the text in `buffer` has no color defined, `default_color` will be
        /// used.
        ///
        /// `origin` allows controlling how the text will be drawn relative to the
        /// coordinate provided in [`render()`](crate::PreparedGraphic::render).
        pub fn draw_text_buffer<Unit>(
            &mut self,
            buffer: &cosmic_text::Buffer,
            default_color: Color,
            origin: TextOrigin<Px>,
            translate: Point<Unit>,
            rotation: Option<Angle>,
            scale: Option<f32>,
        ) where
            Unit: ScreenUnit,
        {
            self.draw_text_buffer_inner(
                Some(buffer),
                default_color,
                origin,
                translate,
                rotation,
                scale,
            );
        }

        /// Prepares the text layout contained in `buffer` to be rendered.
        ///
        /// When the text in `buffer` has no color defined, `default_color` will be
        /// used.
        ///
        /// `origin` allows controlling how the text will be drawn relative to the
        /// coordinate provided in [`render()`](crate::PreparedGraphic::render).
        pub fn draw_measured_text<Unit>(
            &mut self,
            text: &MeasuredText<Unit>,
            origin: TextOrigin<Unit>,
            translate: Point<Unit>,
            rotation: Option<Angle>,
            scale: Option<f32>,
        ) where
            Unit: ScreenUnit,
        {
            let scaling_factor = self.scale;
            let translation = translate.into_px(scaling_factor).cast();
            let origin = match origin {
                TextOrigin::TopLeft => Point::default(),
                TextOrigin::Center => Point::from(text.size).into_px(scaling_factor).cast() / 2,
                TextOrigin::FirstBaseline => Point::new(Px(0), text.ascent.into_px(scaling_factor)),
                TextOrigin::Custom(offset) => offset.into_px(scaling_factor).cast(),
            };
            for (blit, cached, _is_first_line) in &text.glyphs {
                let mut blit = *blit;
                blit.translate_by(-origin);
                render_one_glyph(
                    translation,
                    rotation,
                    scale,
                    blit,
                    cached,
                    self.clip_index,
                    &mut self.data.vertices,
                    &mut self.data.indices,
                    &mut self.data.textures,
                    &mut self.data.commands,
                );
            }
        }

        fn draw_text_buffer_inner<Unit>(
            &mut self,
            buffer: Option<&cosmic_text::Buffer>,
            default_color: Color,
            origin: TextOrigin<Px>,
            translate: Point<Unit>,
            rotation: Option<Angle>,
            scale: Option<f32>,
        ) where
            Unit: ScreenUnit,
        {
            let queue = self.queue;
            let scaling_factor = self.scale;
            let translation = translate.into_px(scaling_factor).cast();
            map_each_glyph(
                buffer,
                default_color,
                origin,
                self.graphics.kludgine,
                queue,
                &mut self.data.glyphs,
                |blit, cached, _is_first_line| {
                    render_one_glyph(
                        translation,
                        rotation,
                        scale,
                        blit,
                        cached,
                        self.clip_index,
                        &mut self.data.vertices,
                        &mut self.data.indices,
                        &mut self.data.textures,
                        &mut self.data.commands,
                    );
                },
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_one_glyph(
        translation: Point<i32>,
        rotation: Option<Angle>,
        scale: Option<f32>,
        blit: TextureBlit<Px>,
        cached: &CachedGlyphHandle,
        clip_index: u32,
        vertices: &mut VertexCollection<i32>,
        indices: &mut Vec<u16>,
        textures: &mut AHashMap<TextureId, Arc<wgpu::BindGroup>>,
        commands: &mut Vec<Command>,
    ) {
        let corners: [u16; 4] = array::from_fn(|index| {
            let vertex = &blit.verticies[index];
            vertices.get_or_insert(Vertex {
                location: vertex.location.into_signed().cast(),
                texture: vertex.texture,
                color: vertex.color,
            })
        });
        let start_index = u32::try_from(indices.len()).expect("too many drawn indices");
        for &index in blit.indices() {
            indices.push(corners[usize::from(index)]);
        }
        let mut flags = Px::flags() | FLAG_TEXTURED;
        if let hash_map::Entry::Vacant(vacant) = textures.entry(cached.texture.id()) {
            vacant.insert(cached.texture.bind_group());
        }

        if cached.is_mask {
            flags |= FLAG_MASKED;
        }
        let scale = scale.map_or(1., |scale| {
            flags |= FLAG_SCALE;
            scale
        });
        let rotation = rotation.map_or(0., |scale| {
            flags |= FLAG_ROTATE;
            scale.into_raidans_f()
        });
        if !translation.is_zero() {
            flags |= FLAG_TRANSLATE;
        }

        let constants = PushConstants {
            flags,
            scale,
            rotation,
            translation,
        };
        let end_index = u32::try_from(indices.len()).expect("too many drawn indices");
        match commands.last_mut() {
            Some(last_command) if last_command.constants == constants => {
                // The last command was from the same texture source, we can stend the previous range to the new end.
                last_command.indices.end = end_index;
            }
            _ => {
                commands.push(Command {
                    clip_index,
                    indices: start_index..end_index,
                    constants,
                    texture: Some(cached.texture.id()),
                });
            }
        }
    }
}

impl Drop for Renderer<'_, '_> {
    fn drop(&mut self) {
        if !self.data.indices.is_empty() {
            if let Some(buffers) = &mut self.data.buffers {
                buffers.vertex.update(
                    &self.data.vertices.vertices,
                    self.graphics.device,
                    self.graphics.queue,
                );
                buffers.index.update(
                    &self.data.indices,
                    self.graphics.device,
                    self.graphics.queue,
                );
            } else {
                // Create new buffers
                self.data.buffers = Some(RenderingBuffers {
                    vertex: DiffableBuffer::new(
                        &self.data.vertices.vertices,
                        wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        self.graphics.device,
                    ),
                    index: DiffableBuffer::new(
                        &self.data.indices,
                        wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                        self.graphics.device,
                    ),
                });
            }
        }
    }
}

/// A composite, multi-operation graphic, created with an easy-to-use
/// [`Renderer`]-driven API.
///
/// The process of preparing individual graphics and then rendering them allows
/// for efficient rendering. The downside is that it can be harder to use, and
/// each [`PreparedGraphic`](crate::PreparedGraphic) contains its own vertex and
/// index buffers.
///
/// This type allows rendering a batch of drawing operations using a
/// [`Renderer`]. Once the renderer is dropped, this type's vertex buffer and
/// index buffer are updated.
#[derive(Default, Debug)]
pub struct Drawing {
    buffers: Option<RenderingBuffers>,
    vertices: VertexCollection<i32>,
    clips: Vec<Rect<UPx>>,
    clip_lookup: AHashMap<Rect<UPx>, u32>,
    indices: Vec<u16>,
    textures: AHashMap<sealed::TextureId, Arc<wgpu::BindGroup>>,
    commands: Vec<Command>,
    #[cfg(feature = "cosmic-text")]
    glyphs: AHashMap<crate::text::PixelAlignedCacheKey, crate::text::CachedGlyphHandle>,
}

#[derive(Debug)]
struct RenderingBuffers {
    vertex: DiffableBuffer<Vertex<i32>>,
    index: DiffableBuffer<u16>,
}

impl Drawing {
    /// Clears the currently prepared graphics and returns a new [`Renderer`] to
    /// prepare new graphics.
    ///
    /// Once the renderer is dropped, this type is ready to be rendered.
    pub fn new_frame<'rendering, 'gfx>(
        &'rendering mut self,
        graphics: &'rendering mut Graphics<'gfx>,
    ) -> Renderer<'rendering, 'gfx> {
        self.commands.clear();
        self.indices.clear();
        self.textures.clear();
        self.vertices.vertex_index_by_id.clear();
        self.vertices.vertices.clear();
        self.clip_lookup.clear();
        self.clips.clear();
        self.get_or_lookup_clip(graphics.clip);
        #[cfg(feature = "cosmic-text")]
        self.glyphs.clear();

        Renderer {
            graphics,
            clip_index: 0,
            data: self,
        }
    }

    fn get_or_lookup_clip(&mut self, clip: ClipRect) -> u32 {
        *self.clip_lookup.entry(clip.0).or_insert_with(|| {
            let id = u32::try_from(self.clips.len()).expect("too many clips");
            self.clips.push(clip.0);
            id
        })
    }

    /// Renders the prepared graphics from the last frame.
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

            let mut current_clip_index = 0;
            let mut current_clip = graphics.clip.0;

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

                if current_clip_index != command.clip_index {
                    current_clip = self.clips[command.clip_index as usize];
                    current_clip_index = command.clip_index;
                    graphics.pass.set_scissor_rect(
                        current_clip.origin.x.0,
                        current_clip.origin.y.0,
                        current_clip.size.width.0,
                        current_clip.size.height.0,
                    );
                }

                let mut constants = command.constants;
                constants.translation +=
                    current_clip.origin.try_cast().expect("clip rect too large");
                if !constants.translation.is_zero() {
                    constants.flags |= FLAG_TRANSLATE;
                }
                graphics.pass.set_push_constants(
                    wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    0,
                    bytemuck::bytes_of(&constants),
                );
                graphics.pass.draw_indexed(command.indices.clone(), 0, 0..1);
            }
        }
    }
}
