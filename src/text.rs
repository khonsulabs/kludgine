use std::collections::hash_map;
use std::sync::{Arc, Mutex, PoisonError};

use ahash::AHashMap;
use cosmic_text::{fontdb, SwashContent};
use figures::units::Px;
use figures::utils::lossy_f32_to_i32;
use figures::{Point, Rect, Size};

use crate::render::Rendering;
use crate::shapes::PathBuilder;
use crate::{CollectedTexture, Color, Graphics, ProtoGraphics, TextureCollection};

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
        insert_fn: impl FnOnce() -> Option<CollectedTexture>,
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
                let texture = insert_fn()?;
                vacant.insert(CachedGlyph {
                    texture,
                    ref_count: 1,
                })
            }
        };
        Some(CachedGlyphHandle {
            key,
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
    ref_count: usize,
}

struct CachedGlyphHandle {
    key: PixelAlignedCacheKey,
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
    pub fn prepare_text(
        &mut self,
        buffer: &cosmic_text::Buffer,
        default_color: Color,
    ) -> PreparedText {
        let mut glyphs = AHashMap::new();
        let mut rendering = Rendering::default();
        let queue = self.queue();
        let mut frame = rendering.new_frame(self);
        let mut path_builder = PathBuilder::new_textured(Point::default(), Point::default());

        let line_height = buffer.metrics().line_height;
        for run in buffer.layout_runs() {
            let run_origin = Point::new(0, run.line_y - line_height);
            for glyph in run.glyphs.iter() {
                let Some(image) = frame
                    .graphics
                    .kludgine.text
                    .swash_cache
                    .get_image(&mut frame.graphics.kludgine.text.fonts, glyph.cache_key) else { continue };
                if image.placement.width == 0 || image.placement.height == 0 {
                    continue;
                }

                let mut color = glyph.color_opt.map_or(default_color, Color::from);

                let Some(cached) = frame.graphics.kludgine.text.glyphs.get_or_insert(
                    glyph.cache_key.into(),
                    || match image.content {
                        SwashContent::Mask => {
                            Some(frame.graphics.kludgine.text.alpha_text_atlas.push_texture(
                                &image.data,
                                wgpu::ImageDataLayout {
                                    offset: 0,
                                    bytes_per_row: Some(image.placement.width),
                                    rows_per_image: None,
                                },
                                Size::new(image.placement.width, image.placement.height),
                                queue,
                            ))
                        }
                        SwashContent::Color => {
                            // Set the color to full white to avoid mixing.
                            color = Color::WHITE;
                            Some(frame.graphics.kludgine.text.color_text_atlas.push_texture(
                                &image.data,
                                wgpu::ImageDataLayout {
                                    offset: 0,
                                    bytes_per_row: Some(image.placement.width * 4),
                                    rows_per_image: None,
                                },
                                Size::new(image.placement.width, image.placement.height),
                                queue,
                            ))
                        }
                        SwashContent::SubpixelMask => None,
                    },
                ) else { continue };

                let (source_top_left, source_bottom_right) = cached.texture.region.extents();
                let (dest_top_left, dest_bottom_right) = Rect::<Px>::new(
                    run_origin
                        + Point::new(
                            image.placement.left,
                            lossy_f32_to_i32(line_height) - image.placement.top,
                        ),
                    Size::new(
                        i32::try_from(image.placement.width).expect("width out of range of i32"),
                        i32::try_from(image.placement.height).expect("height out of range of i32"),
                    ),
                )
                .extents();
                // TODO we should be able to reuse this builder.
                path_builder.reset(dest_top_left, source_top_left);
                let path = path_builder
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
                let shape = path.fill(color);
                frame.draw_textured_shape(
                    &shape,
                    &cached.texture,
                    Point::new(glyph.x, glyph.y_int),
                    None,
                    None,
                );
                path_builder = PathBuilder::from(path);
                glyphs
                    .entry(glyph.cache_key.into())
                    .or_insert_with(|| cached);
            }
        }
        drop(frame);

        PreparedText {
            graphic: rendering,
            _glyphs: glyphs,
        }
    }
}

pub struct PreparedText {
    graphic: Rendering,
    _glyphs: AHashMap<PixelAlignedCacheKey, CachedGlyphHandle>,
}

impl std::ops::Deref for PreparedText {
    type Target = Rendering;

    fn deref(&self) -> &Self::Target {
        &self.graphic
    }
}

impl std::ops::DerefMut for PreparedText {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.graphic
    }
}
