use std::collections::{hash_map, HashMap};
use std::ops::{Deref, DerefMut, Range};
use std::sync::Arc;

use figures::units::{Px, UPx};
use figures::{Angle, IntoSigned, Point, Rect, ScreenScale, ScreenUnit, Size, UnscaledUnit, Zero};
use intentional::CastInto;

use crate::buffer::DiffableBuffer;
use crate::pipeline::{
    PushConstants, ShaderScalable, Vertex, FLAG_MASKED, FLAG_ROTATE, FLAG_SCALE, FLAG_TEXTURED,
    FLAG_TRANSLATE,
};
use crate::shapes::Shape;
use crate::{
    sealed, Assert, ClipGuard, ClipRect, Clipped, Color, DefaultHasher, Drawable, Graphics,
    RenderingGraphics, ShapeSource, Texture, TextureBlit, TextureSource, VertexCollection,
};

/// An easy-to-use graphics renderer that batches operations on the GPU
/// automatically.
///
/// Using the draw operations on this type don't immediately draw. Instead, once
/// this type is dropped, the [`Drawing`] that created this renderer will be
/// updated with the new drawing instructions. All of the pending operations can
/// be drawn using [`Drawing::render`].
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
    pub fn draw_shape<'shape, Unit>(
        &mut self,
        shape: impl Into<Drawable<&'shape Shape<Unit, false>, Unit>>,
    ) where
        Unit: Zero + ShaderScalable + ScreenUnit + figures::Unit + Copy,
    {
        self.inner_draw(&shape.into(), Option::<&Texture>::None);
    }

    /// Draws `texture` at `destination`, scaling as necessary.
    pub fn draw_texture<Unit>(&mut self, texture: &impl TextureSource, destination: Rect<Unit>)
    where
        Unit: figures::Unit + ScreenUnit + ShaderScalable,
        i32: From<<Unit as IntoSigned>::Signed>,
    {
        self.draw_textured_shape(
            &TextureBlit::new(texture.default_rect(), destination, Color::WHITE),
            texture,
        );
    }

    /// Draws `texture` at `destination`.
    pub fn draw_texture_at<Unit>(&mut self, texture: &impl TextureSource, destination: Point<Unit>)
    where
        Unit: figures::Unit + ScreenUnit + ShaderScalable,
        i32: From<<Unit as IntoSigned>::Signed>,
    {
        let texture_rect = texture.default_rect();
        let scaled_size = Size::<Unit>::from_upx(texture_rect.size, self.scale);
        self.draw_textured_shape(
            &TextureBlit::new(
                texture_rect,
                Rect::new(destination, scaled_size),
                Color::WHITE,
            ),
            texture,
        );
    }

    /// Draws a shape that was created with texture coordinates, applying the
    /// provided texture.
    pub fn draw_textured_shape<'shape, Unit, Shape>(
        &mut self,
        shape: impl Into<Drawable<&'shape Shape, Unit>>,
        texture: &impl TextureSource,
    ) where
        Unit: Zero + ShaderScalable + ScreenUnit + figures::Unit + Copy,
        i32: From<<Unit as IntoSigned>::Signed>,
        Shape: ShapeSource<Unit, true> + 'shape,
    {
        self.inner_draw(&shape.into(), Some(texture));
    }

    fn inner_draw<Shape, Unit, const TEXTURED: bool>(
        &mut self,
        shape: &Drawable<&'_ Shape, Unit>,
        texture: Option<&impl TextureSource>,
    ) where
        Unit: Zero + ShaderScalable + ScreenUnit + figures::Unit + Copy,
        Shape: ShapeSource<Unit, TEXTURED>,
    {
        // Merge the vertices into the graphics
        let vertices = shape.source.vertices();
        let mut vertex_map = Vec::with_capacity(vertices.len());
        for vertex in vertices {
            let vertex = Vertex {
                location: vertex.location.map(|u| u.into_unscaled().cast_into()),
                texture: vertex.texture,
                color: vertex.color,
            };
            let index = self.data.vertices.get_or_insert(vertex);
            vertex_map.push(index);
        }

        let first_index_drawn = self.data.indices.len();
        for &vertex_index in shape.source.indices() {
            self.data
                .indices
                .push(vertex_map[usize::try_from(vertex_index).assert("too many drawn indices")]);
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
                entry.insert(texture.bind_group(self.graphics));
            }
            Some(id)
        } else {
            None
        };
        let scale = shape.scale.map_or(1., |scale| {
            flags |= FLAG_SCALE;
            scale
        });
        let rotation = shape.rotation.map_or(0., |scale| {
            flags |= FLAG_ROTATE;
            scale.into_raidans_f()
        });
        if !shape.translation.is_zero() {
            flags |= FLAG_TRANSLATE;
        }

        let constants = PushConstants {
            flags,
            scale,
            rotation,
            opacity: shape.opacity.unwrap_or(1.),
            translation: shape
                .translation
                .into_px(self.graphics.scale())
                .map(Px::into_unscaled),
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
    /// during [`render()`](Drawing::render).
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
        self.push_clip(clip);
        self.clip_index = self.data.get_or_lookup_clip(self.clip.current);

        ClipGuard { clipped: self }
    }
}

impl Clipped for Renderer<'_, '_> {
    fn push_clip(&mut self, clip: Rect<UPx>) {
        self.clip.push_clip(clip);
        self.clip_index = self.data.get_or_lookup_clip(self.clip.current);
    }

    fn pop_clip(&mut self) {
        self.graphics.pop_clip();
        self.clip_index = self.data.get_or_lookup_clip(self.clip.current);
    }
}

impl sealed::Clipped for Renderer<'_, '_> {}

#[cfg(feature = "cosmic-text")]
mod text {
    use std::array;
    use std::collections::{hash_map, HashMap};
    use std::sync::Arc;

    use figures::units::Px;
    use figures::{Fraction, ScreenScale, ScreenUnit, UnscaledUnit};
    use intentional::Assert;

    use super::{
        Angle, Color, Command, IntoSigned, Point, PushConstants, Renderer, Vertex, Zero,
        FLAG_MASKED, FLAG_ROTATE, FLAG_SCALE, FLAG_TEXTURED, FLAG_TRANSLATE,
    };
    use crate::sealed::{ShaderScalableSealed, ShapeSource, TextureId, TextureSource};
    use crate::text::{
        map_each_glyph, measure_text, CachedGlyphHandle, MeasuredText, Text, TextOrigin,
    };
    use crate::{
        DefaultHasher, Drawable, KludgineGraphics, ProtoGraphics, TextureBlit, VertexCollection,
    };

    impl<'gfx> Renderer<'_, 'gfx> {
        /// Measures `text` using the current text settings.
        ///
        /// `default_color` does not affect the
        pub fn measure_text<'a, Unit>(
            &mut self,
            text: impl Into<Text<'a, Unit>>,
        ) -> MeasuredText<Unit>
        where
            Unit: figures::ScreenUnit,
        {
            let text = text.into();
            let scale = self.graphics.scale;
            self.update_scratch_buffer(text.text, text.wrap_at.map(|width| width.into_px(scale)));
            measure_text::<Unit, true>(
                None,
                text.color,
                self.graphics.kludgine,
                self.graphics.device,
                self.graphics.queue,
                &mut self.data.glyphs,
            )
        }

        /// Draws `text` using the current text settings.
        pub fn draw_text<'a, Unit, Source>(&mut self, text: Source)
        where
            Unit: ScreenUnit,
            Source: Into<Drawable<Text<'a, Unit>, Unit>>,
        {
            let text = text.into();
            self.graphics.kludgine.update_scratch_buffer(
                text.source.text,
                text.source
                    .wrap_at
                    .map(|width| width.into_px(self.graphics.scale)),
            );
            self.draw_text_buffer_inner(
                None,
                text.source.color,
                text.source.origin.into_px(self.scale()),
                text.translation,
                text.rotation,
                text.scale,
                text.opacity,
            );
        }

        /// Prepares the text layout contained in `buffer` to be rendered.
        ///
        /// When the text in `buffer` has no color defined, `default_color` will be
        /// used.
        ///
        /// `origin` allows controlling how the text will be drawn relative to the
        /// coordinate provided in [`render()`](crate::PreparedGraphic::render).
        pub fn draw_text_buffer<'a, Unit>(
            &mut self,
            buffer: impl Into<Drawable<&'a cosmic_text::Buffer, Unit>>,
            default_color: Color,
            origin: TextOrigin<Px>,
        ) where
            Unit: ScreenUnit,
        {
            let buffer = buffer.into();
            self.draw_text_buffer_inner(
                Some(buffer.source),
                default_color,
                origin,
                buffer.translation,
                buffer.rotation,
                buffer.scale,
                buffer.opacity,
            );
        }

        /// Measures `buffer` and caches the results using `default_color` when
        /// the buffer has no color associated with text.
        pub fn measure_text_buffer<Unit>(
            &mut self,
            buffer: &cosmic_text::Buffer,
            default_color: Color,
        ) -> MeasuredText<Unit>
        where
            Unit: figures::ScreenUnit,
        {
            measure_text::<Unit, true>(
                Some(buffer),
                default_color,
                self.graphics.kludgine,
                self.graphics.device,
                self.graphics.queue,
                &mut self.data.glyphs,
            )
        }

        /// Prepares the text layout contained in `buffer` to be rendered.
        ///
        /// When the text in `buffer` has no color defined, `default_color` will be
        /// used.
        ///
        /// `origin` allows controlling how the text will be drawn relative to the
        /// coordinate provided in [`render()`](crate::PreparedGraphic::render).
        pub fn draw_measured_text<'a, Unit>(
            &mut self,
            text: impl Into<Drawable<&'a MeasuredText<Unit>, Unit>>,
            origin: TextOrigin<Unit>,
        ) where
            Unit: ScreenUnit,
        {
            let text = text.into();
            let scaling_factor = self.scale;
            let translation = text.translation;
            let origin = match origin {
                TextOrigin::TopLeft => Point::default(),
                TextOrigin::Center => Point::from(text.source.size).into_px(scaling_factor) / 2,
                TextOrigin::FirstBaseline => {
                    Point::new(Px::ZERO, text.source.ascent.into_px(scaling_factor))
                }
                TextOrigin::Custom(offset) => offset.into_px(scaling_factor),
            };
            for glyph in &text.source.glyphs {
                let mut blit = glyph.blit;
                blit.translate_by(-origin);
                render_one_glyph(
                    translation,
                    text.rotation,
                    text.scale,
                    text.opacity,
                    blit,
                    &glyph.cached,
                    self.clip_index,
                    scaling_factor,
                    self.graphics,
                    &mut self.data.vertices,
                    &mut self.data.indices,
                    &mut self.data.textures,
                    &mut self.data.commands,
                );
            }
        }

        #[allow(clippy::too_many_arguments)]
        fn draw_text_buffer_inner<Unit>(
            &mut self,
            buffer: Option<&cosmic_text::Buffer>,
            default_color: Color,
            origin: TextOrigin<Px>,
            translation: Point<Unit>,
            rotation: Option<Angle>,
            scale: Option<f32>,
            opacity: Option<f32>,
        ) where
            Unit: ScreenUnit,
        {
            let scaling_factor = self.scale;
            map_each_glyph(
                buffer,
                default_color,
                origin,
                self.graphics.kludgine,
                self.graphics.device,
                self.graphics.queue,
                &mut self.data.glyphs,
                |blit, cached, _glyph, _is_first_line, _baseline, _line_w, kludgine| {
                    render_one_glyph(
                        translation,
                        rotation,
                        scale,
                        opacity,
                        blit,
                        cached,
                        self.clip_index,
                        scaling_factor,
                        &ProtoGraphics::new(self.graphics.device, self.graphics.queue, kludgine),
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
    fn render_one_glyph<Unit>(
        translation: Point<Unit>,
        rotation: Option<Angle>,
        scale: Option<f32>,
        opacity: Option<f32>,
        blit: TextureBlit<Px>,
        cached: &CachedGlyphHandle,
        clip_index: u32,
        dpi_scale: Fraction,
        graphics: &impl KludgineGraphics,
        vertices: &mut VertexCollection<i32>,
        indices: &mut Vec<u32>,
        textures: &mut HashMap<TextureId, Arc<wgpu::BindGroup>, DefaultHasher>,
        commands: &mut Vec<Command>,
    ) where
        Unit: ScreenUnit,
    {
        let translation = translation.into_px(dpi_scale).map(Px::into_unscaled);
        let corners: [u32; 4] = array::from_fn(|index| {
            let vertex = &blit.verticies[index];
            vertices.get_or_insert(Vertex {
                location: vertex.location.into_signed().map(Px::into_unscaled),
                texture: vertex.texture,
                color: vertex.color,
            })
        });
        let start_index = u32::try_from(indices.len()).expect("too many drawn indices");
        for &index in blit.indices() {
            indices.push(corners[usize::try_from(index).assert("too many drawn indices")]);
        }
        let mut flags = Px::flags() | FLAG_TEXTURED;
        if let hash_map::Entry::Vacant(vacant) = textures.entry(cached.texture.id()) {
            vacant.insert(cached.texture.bind_group(graphics));
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
            opacity: opacity.unwrap_or(1.),
        };
        let end_index = u32::try_from(indices.len()).expect("too many drawn indices");
        match commands.last_mut() {
            Some(last_command)
                if clip_index == last_command.clip_index
                    && last_command.texture == Some(cached.texture.id())
                    && last_command.constants == constants =>
            {
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
    clip_lookup: HashMap<Rect<UPx>, u32, DefaultHasher>,
    indices: Vec<u32>,
    textures: HashMap<sealed::TextureId, Arc<wgpu::BindGroup>, DefaultHasher>,
    commands: Vec<Command>,
    #[cfg(feature = "cosmic-text")]
    glyphs: HashMap<cosmic_text::CacheKey, crate::text::CachedGlyphHandle, DefaultHasher>,
}

#[derive(Debug)]
struct RenderingBuffers {
    vertex: DiffableBuffer<Vertex<i32>>,
    index: DiffableBuffer<u32>,
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
        self.get_or_lookup_clip(graphics.clip.current);
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
                .set_index_buffer(buffers.index.as_slice(), wgpu::IndexFormat::Uint32);

            let mut current_clip_index = 0;
            let mut current_clip = graphics.clip.current.0;

            for command in &self.commands {
                if let Some(texture_id) = &command.texture {
                    if current_texture_id != Some(*texture_id) {
                        needs_texture_binding = false;
                        current_texture_id = Some(*texture_id);
                        graphics.pass.set_bind_group(
                            0,
                            self.textures.get(texture_id).assert("texture missing"),
                            &[],
                        );
                    }
                } else if needs_texture_binding {
                    needs_texture_binding = false;
                    current_texture_id = None;
                    graphics
                        .pass
                        .set_bind_group(0, &graphics.kludgine.default_bindings, &[]);
                }

                if current_clip_index != command.clip_index {
                    current_clip_index = command.clip_index;
                    current_clip = self.clips[command.clip_index as usize];
                    if current_clip.size.is_zero() {
                        continue;
                    }

                    graphics.pass.set_scissor_rect(
                        current_clip.origin.x.into(),
                        current_clip.origin.y.into(),
                        current_clip.size.width.into(),
                        current_clip.size.height.into(),
                    );
                }

                let mut constants = command.constants;
                constants.translation += current_clip.origin.into_signed().map(Px::into_unscaled);
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
