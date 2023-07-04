use std::collections::hash_map;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use ahash::AHashMap;
use cosmic_text::{fontdb, SwashContent};
use figures::traits::FloatConversion;
use figures::units::Px;
use figures::utils::lossy_f32_to_i32;
use figures::{Point, Rect, Size};

use crate::buffer::Buffer;
use crate::pipeline::{PreparedCommand, Vertex};
use crate::sealed::TextureSource;
use crate::{
    CollectedTexture, Color, Graphics, PreparedGraphic, ProtoGraphics, TextureCollection,
    VertexCollection,
};

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
struct PixelAlignedCacheKey {
    /// Font ID
    pub font_id: fontdb::ID,
    /// Glyph ID
    pub glyph_id: u16,
    /// `f32` bits of font size
    pub font_size_bits: u32,
}

impl From<cosmic_text::CacheKey> for PixelAlignedCacheKey {
    fn from(key: cosmic_text::CacheKey) -> Self {
        Self {
            font_id: key.font_id,
            glyph_id: key.glyph_id,
            font_size_bits: key.font_size_bits,
        }
    }
}

pub struct TextSystem {
    pub fonts: cosmic_text::FontSystem,
    pub swash_cache: cosmic_text::SwashCache,
    pub alpha_text_atlas: TextureCollection,
    pub color_text_atlas: TextureCollection,
    glyphs: GlyphCache,
}

impl TextSystem {
    pub(crate) fn new(graphics: &ProtoGraphics<'_>) -> Self {
        Self {
            alpha_text_atlas: TextureCollection::new_generic(
                Size::new(512, 512),
                wgpu::TextureFormat::R8Unorm,
                graphics,
            ),
            color_text_atlas: TextureCollection::new_generic(
                Size::new(512, 512),
                wgpu::TextureFormat::Rgba8Unorm,
                graphics,
            ),
            fonts: cosmic_text::FontSystem::new(),
            swash_cache: cosmic_text::SwashCache::new(),
            glyphs: GlyphCache::default(),
        }
    }

    pub fn new_frame(&mut self) {
        self.glyphs.clear_unused();
    }
}

#[derive(Default, Clone)]
struct GlyphCache {
    glyphs: Arc<Mutex<AHashMap<PixelAlignedCacheKey, CachedGlyph>>>,
}

impl GlyphCache {
    fn get_or_insert(
        &mut self,
        key: PixelAlignedCacheKey,
        insert_fn: impl FnOnce() -> Option<(CollectedTexture, bool)>,
    ) -> Option<CachedGlyphHandle> {
        let mut data = self
            .glyphs
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g);
        let cached = match data.entry(key) {
            hash_map::Entry::Occupied(cached) => {
                let cached = cached.into_mut();
                cached.ref_count += 1;
                cached
            }
            hash_map::Entry::Vacant(vacant) => {
                let (texture, is_mask) = insert_fn()?;
                vacant.insert(CachedGlyph {
                    texture,
                    is_mask,
                    ref_count: 1,
                })
            }
        };
        Some(CachedGlyphHandle {
            key,
            is_mask: cached.is_mask,
            cache: self.clone(),
            texture: cached.texture.clone(),
        })
    }

    fn clear_unused(&mut self) {
        let mut data = self
            .glyphs
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g);
        data.retain(|_, glyph| glyph.ref_count > 0);
    }
}

struct CachedGlyph {
    texture: CollectedTexture,
    is_mask: bool,
    ref_count: usize,
}

struct CachedGlyphHandle {
    key: PixelAlignedCacheKey,
    is_mask: bool,
    cache: GlyphCache,
    texture: CollectedTexture,
}

impl Drop for CachedGlyphHandle {
    fn drop(&mut self) {
        let mut data = self
            .cache
            .glyphs
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g);
        let cached = data.get_mut(&self.key).expect("cached glyph missing");
        cached.ref_count -= 1;
    }
}

impl<'gfx> Graphics<'gfx> {
    #[allow(clippy::too_many_lines)]
    pub fn prepare_text(
        &mut self,
        buffer: &cosmic_text::Buffer,
        default_color: Color,
        origin: TextOrigin,
    ) -> PreparedText {
        let mut glyphs = AHashMap::new();
        let queue = self.queue();

        let line_height = buffer.metrics().line_height;
        let mut verticies = VertexCollection::default();
        let mut indices = Vec::new();
        let mut commands = Vec::<PreparedCommand>::new();

        let relative_to: Point<Px> = match origin {
            TextOrigin::TopLeft => Point::default(),
            TextOrigin::Center => {
                let (x, y) = buffer
                    .layout_runs()
                    .map(|run| (run.line_w, run.line_y))
                    .fold((0f32, 0f32), |(x, y), (run_x, run_y)| {
                        (x.max(run_x), y.max(run_y))
                    });
                Point::new(Px::from_float(x) / 2, Px::from_float(y) / 2)
            }
            TextOrigin::FirstBaseline => Point::new(0, Px::from_float(buffer.metrics().font_size)),
        };

        for run in buffer.layout_runs() {
            let run_origin = Point::new(0, run.line_y - line_height);
            for glyph in run.glyphs.iter() {
                let Some(image) = self
                    .kludgine.text
                    .swash_cache
                    .get_image(&mut self.kludgine.text.fonts, glyph.cache_key) else { continue };
                if image.placement.width == 0 || image.placement.height == 0 {
                    continue;
                }

                let mut color = glyph.color_opt.map_or(default_color, Color::from);

                let Some(cached) = self.kludgine.text.glyphs.get_or_insert(
                    glyph.cache_key.into(),
                    || match image.content {
                        SwashContent::Mask => {
                            Some((self.kludgine.text.alpha_text_atlas.push_texture(
                                &image.data,
                                wgpu::ImageDataLayout {
                                    offset: 0,
                                    bytes_per_row: Some(image.placement.width),
                                    rows_per_image: None,
                                },
                                Size::new(image.placement.width, image.placement.height),
                                queue,
                            ), true))
                        }
                        SwashContent::Color => {
                            // Set the color to full white to avoid mixing.
                            color = Color::WHITE;
                            Some((self.kludgine.text.color_text_atlas.push_texture(
                                &image.data,
                                wgpu::ImageDataLayout {
                                    offset: 0,
                                    bytes_per_row: Some(image.placement.width * 4),
                                    rows_per_image: None,
                                },
                                Size::new(image.placement.width, image.placement.height),
                                queue,
                            ), false))
                        }
                        SwashContent::SubpixelMask => None,
                    },
                ) else { continue };

                let (source_top_left, source_bottom_right) = cached.texture.region.extents();
                let (dest_top_left, dest_bottom_right) = Rect::<Px>::new(
                    Point::<Px>::new(glyph.x, glyph.y_int)
                        + run_origin
                        + Point::new(
                            image.placement.left,
                            lossy_f32_to_i32(line_height) - image.placement.top,
                        )
                        - relative_to,
                    Size::new(
                        i32::try_from(image.placement.width).expect("width out of range of i32"),
                        i32::try_from(image.placement.height).expect("height out of range of i32"),
                    ),
                )
                .extents();

                let top_left = verticies.get_or_insert(Vertex {
                    location: dest_top_left,
                    texture: source_top_left,
                    color,
                });
                let top_right = verticies.get_or_insert(Vertex {
                    location: Point::new(dest_bottom_right.x, dest_top_left.y),
                    texture: Point::new(source_bottom_right.x, source_top_left.y),
                    color,
                });
                let bottom_left = verticies.get_or_insert(Vertex {
                    location: dest_bottom_right,
                    texture: source_bottom_right,
                    color,
                });
                let bottom_right = verticies.get_or_insert(Vertex {
                    location: Point::new(dest_top_left.x, dest_bottom_right.y),
                    texture: Point::new(source_top_left.x, source_bottom_right.y),
                    color,
                });
                let start_index = u32::try_from(indices.len()).expect("too many drawn indices");
                indices.push(top_right);
                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(bottom_right);
                let end_index = u32::try_from(indices.len()).expect("too many drawn indices");
                match commands.last_mut() {
                    Some(last_command) if last_command.is_mask == cached.is_mask => {
                        // The last command was from the same texture source, we can stend the previous range to the new end.
                        last_command.indices.end = end_index;
                    }
                    _ => {
                        commands.push(PreparedCommand {
                            indices: start_index..end_index,
                            is_mask: cached.is_mask,
                            binding: Some(cached.texture.bind_group()),
                        });
                    }
                }

                glyphs
                    .entry(glyph.cache_key.into())
                    .or_insert_with(|| cached);
            }
        }

        PreparedText {
            graphic: PreparedGraphic {
                vertices: Buffer::new(&verticies.vertices, wgpu::BufferUsages::VERTEX, self.device),
                indices: Buffer::new(&indices, wgpu::BufferUsages::INDEX, self.device),
                commands,
            },
            _glyphs: glyphs,
        }
    }
}

pub struct PreparedText {
    graphic: PreparedGraphic<Px>,
    _glyphs: AHashMap<PixelAlignedCacheKey, CachedGlyphHandle>,
}

impl fmt::Debug for PreparedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.graphic.fmt(f)
    }
}

impl std::ops::Deref for PreparedText {
    type Target = PreparedGraphic<Px>;

    fn deref(&self) -> &Self::Target {
        &self.graphic
    }
}

impl std::ops::DerefMut for PreparedText {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.graphic
    }
}

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub enum TextOrigin {
    #[default]
    TopLeft,
    Center,
    FirstBaseline,
}
