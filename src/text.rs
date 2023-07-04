use std::collections::hash_map;

use ahash::AHashMap;
use cosmic_text::{fontdb, SwashContent};
use figures::{lossy_f32_to_i32, Pixels, Point, Rect, Size};

use crate::render::Rendering;
use crate::shapes::PathBuilder;
use crate::{CollectedTexture, Color, Graphics};

#[derive(Debug, Eq, PartialEq, Hash)]
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

impl<'gfx> Graphics<'gfx> {
    pub fn prepare_text(
        &mut self,
        buffer: &cosmic_text::Buffer,
        default_color: Color,
    ) -> PreparedText {
        let mut glyphs = AHashMap::<PixelAlignedCacheKey, CollectedTexture>::new();
        let mut rendering = Rendering::default();
        let queue = self.queue();
        let mut frame = rendering.new_frame(self);
        let mut path_builder = PathBuilder::new_textured(Point::default(), Point::default());

        let line_height = buffer.metrics().line_height;
        for run in buffer.layout_runs() {
            let run_origin = Point::new(0, run.line_y);
            for glyph in run.glyphs.iter() {
                let Some(image) = frame
                    .graphics
                    .kludgine
                    .swash_cache
                    .get_image(&mut frame.graphics.kludgine.fonts, glyph.cache_key) else { continue };
                if image.placement.width == 0 || image.placement.height == 0 {
                    continue;
                }

                let mut color = glyph.color_opt.map_or(default_color, Color::from);

                let texture = match glyphs.entry(glyph.cache_key.into()) {
                    hash_map::Entry::Occupied(texture) => texture.get().clone(),
                    hash_map::Entry::Vacant(vacant) => {
                        let texture = match image.content {
                            SwashContent::Mask => {
                                frame.graphics.kludgine.alpha_text_atlas.push_texture(
                                    &image.data,
                                    wgpu::ImageDataLayout {
                                        offset: 0,
                                        bytes_per_row: Some(image.placement.width),
                                        rows_per_image: None,
                                    },
                                    Size::new(image.placement.width, image.placement.height),
                                    queue,
                                )
                            }
                            SwashContent::Color => {
                                // Set the color to full white to avoid mixing.
                                color = Color::WHITE;
                                frame.graphics.kludgine.color_text_atlas.push_texture(
                                    &image.data,
                                    wgpu::ImageDataLayout {
                                        offset: 0,
                                        bytes_per_row: Some(image.placement.width * 4),
                                        rows_per_image: None,
                                    },
                                    Size::new(image.placement.width, image.placement.height),
                                    queue,
                                )
                            }
                            SwashContent::SubpixelMask => continue,
                        };
                        vacant.insert(texture).clone()
                    }
                };

                let (source_top_left, source_bottom_right) = texture.region.extents();
                let (dest_top_left, dest_bottom_right) = Rect::<Pixels>::new(
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
                    &texture,
                    Point::new(glyph.x, glyph.y_int),
                    None,
                    None,
                );
                path_builder = PathBuilder::from(path);
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
    _glyphs: AHashMap<PixelAlignedCacheKey, CollectedTexture>,
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