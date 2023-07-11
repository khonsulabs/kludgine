use std::collections::hash_map;
use std::fmt::{self, Debug};
use std::ops::Sub;
use std::sync::{Arc, Mutex, PoisonError};

use ahash::AHashMap;
use cosmic_text::{fontdb, Attrs, AttrsOwned, SwashContent};
use figures::traits::{FloatConversion, ScreenScale};
use figures::units::{Lp, Px};
use figures::utils::lossy_f32_to_i32;
use figures::{Fraction, Point, Rect, Size};

use crate::buffer::Buffer;
use crate::pipeline::PreparedCommand;
use crate::sealed::{ShapeSource, TextureSource};
use crate::{
    CollectedTexture, Color, Graphics, Kludgine, PreparedGraphic, ProtoGraphics, TextureBlit,
    TextureCollection, VertexCollection,
};

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub(crate) struct PixelAlignedCacheKey {
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

pub(crate) struct TextSystem {
    pub fonts: cosmic_text::FontSystem,
    pub swash_cache: cosmic_text::SwashCache,
    pub alpha_text_atlas: TextureCollection,
    pub color_text_atlas: TextureCollection,
    pub scratch: Option<cosmic_text::Buffer>,
    pub font_size: Lp,
    pub line_height: Lp,
    pub attrs: AttrsOwned,
    glyphs: GlyphCache,
}

impl TextSystem {
    pub(crate) fn new(graphics: &ProtoGraphics<'_>) -> Self {
        let fonts = cosmic_text::FontSystem::new();

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
            swash_cache: cosmic_text::SwashCache::new(),
            scratch: None,
            fonts,
            font_size: Lp::points(12),
            line_height: Lp::points(14),
            glyphs: GlyphCache::default(),
            attrs: AttrsOwned::new(Attrs::new().color(Color::WHITE.into())),
        }
    }

    pub fn new_frame(&mut self) {
        self.glyphs.clear_unused();
    }

    fn metrics(&self, scale: Fraction) -> cosmic_text::Metrics {
        let font_size = self.font_size.into_px(scale);
        let line_height = self.line_height.into_px(scale);

        cosmic_text::Metrics::new(font_size.into(), line_height.into())
    }

    pub fn set_font_size(&mut self, size: Lp, scale: Fraction) {
        self.font_size = size;
        self.update_buffer_metrics(scale);
    }

    pub fn set_line_height(&mut self, size: Lp, scale: Fraction) {
        self.line_height = size;
        self.update_buffer_metrics(scale);
    }

    pub fn scale_changed(&mut self, scale: Fraction) {
        self.update_buffer_metrics(scale);
    }

    fn update_buffer_metrics(&mut self, scale: Fraction) {
        let metrics = self.metrics(scale);
        if let Some(buffer) = &mut self.scratch {
            buffer.set_metrics(&mut self.fonts, metrics);
        }
    }

    pub fn update_scratch_buffer(&mut self, text: &str, scale: Fraction) {
        if self.scratch.is_none() {
            let metrics = self.metrics(scale);
            let mut buffer = cosmic_text::Buffer::new(&mut self.fonts, metrics);
            buffer.set_size(&mut self.fonts, f32::MAX, f32::MAX);
            self.scratch = Some(buffer);
        }

        let scratch = self.scratch.as_mut().expect("initialized above");
        scratch.set_text(&mut self.fonts, text, self.attrs.as_attrs());
        scratch.shape_until_scroll(&mut self.fonts);
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

pub(crate) struct CachedGlyphHandle {
    key: PixelAlignedCacheKey,
    pub is_mask: bool,
    cache: GlyphCache,
    pub texture: CollectedTexture,
}

impl Debug for CachedGlyphHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedGlyphHandle")
            .field("key", &self.key)
            .field("is_mask", &self.is_mask)
            .finish()
    }
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
    /// Prepares the text layout contained in `buffer` to be rendered.
    ///
    /// When the text in `buffer` has no color defined, `default_color` will be
    /// used.
    ///
    /// `origin` allows controlling how the text will be drawn relative to the
    /// coordinate provided in [`render()`](PreparedGraphic::render).
    pub fn prepare_text(
        &mut self,
        buffer: &cosmic_text::Buffer,
        default_color: Color,
        origin: TextOrigin<Px>,
    ) -> PreparedText {
        let mut glyphs = AHashMap::new();
        let mut verticies = VertexCollection::default();
        let mut indices = Vec::new();
        let mut commands = Vec::<PreparedCommand>::new();

        map_each_glyph(
            Some(buffer),
            default_color,
            origin,
            self.kludgine,
            self.queue,
            &mut glyphs,
            |blit, cached| {
                let mut corners = [0; 4];
                for (&corner, index) in blit.vertices().iter().zip(corners.iter_mut()) {
                    *index = verticies.get_or_insert(corner);
                }
                let start_index = u32::try_from(indices.len()).expect("too many drawn indices");
                for &index in blit.indices() {
                    indices.push(corners[usize::from(index)]);
                }
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
            },
        );

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

#[allow(clippy::too_many_lines)]
pub(crate) fn map_each_glyph(
    buffer: Option<&cosmic_text::Buffer>,
    default_color: Color,
    origin: TextOrigin<Px>,
    kludgine: &mut Kludgine,
    queue: &wgpu::Queue,
    glyphs: &mut AHashMap<PixelAlignedCacheKey, CachedGlyphHandle>,
    mut map: impl for<'a> FnMut(TextureBlit<Px>, &'a CachedGlyphHandle),
) {
    let buffer = buffer.unwrap_or_else(|| kludgine.text.scratch.as_ref().expect("no buffer"));
    let line_height = buffer.metrics().line_height;

    let relative_to = match origin {
        TextOrigin::Custom(point) => point,
        TextOrigin::TopLeft => Point::default(),
        TextOrigin::Center => {
            let (min_x, min_y, max_x, max_y) = buffer
                .layout_runs()
                .flat_map(|run| {
                    run.glyphs.iter().map(move |glyph| {
                        (
                            glyph.x,
                            glyph.x + glyph.w,
                            run.line_y - line_height,
                            run.line_y + glyph.y_offset,
                        )
                    })
                })
                .fold(
                    (f32::MAX, f32::MAX, 0f32, 0f32),
                    |(min_x, min_y, max_x, max_y), (run_min_x, run_max_x, run_min_y, run_max_y)| {
                        (
                            min_x.min(run_min_x),
                            min_y.min(run_min_y),
                            max_x.max(run_max_x),
                            max_y.max(run_max_y),
                        )
                    },
                );
            let x = (max_x + min_x) / 2.;
            let y = (max_y + min_y) / 2.;
            Point {
                x: Px::from_float(x),
                y: Px::from_float(y),
            }
        }
        TextOrigin::FirstBaseline => Point::new(Px(0), Px::from_float(buffer.metrics().font_size)),
    };

    for run in buffer.layout_runs() {
        let run_origin = Point::new(0., run.line_y - line_height);
        for glyph in run.glyphs.iter() {
            let Some(image) = kludgine.text
                .swash_cache
                .get_image(&mut kludgine.text.fonts, glyph.cache_key) else { continue };
            if image.placement.width == 0 || image.placement.height == 0 {
                continue;
            }

            let mut color = glyph.color_opt.map_or(default_color, Color::from);

            let Some(cached) = kludgine.text.glyphs.get_or_insert(
                glyph.cache_key.into(),
                || match image.content {
                    SwashContent::Mask => {
                        Some((kludgine.text.alpha_text_atlas.push_texture(
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
                        Some((kludgine.text.color_text_atlas.push_texture(
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

            let blit = TextureBlit::new(
                cached.texture.region,
                Rect::new(
                    (Point::new(glyph.x, glyph.y_offset) + run_origin).cast::<Px>()
                        + Point::new(
                            image.placement.left,
                            lossy_f32_to_i32(line_height) - image.placement.top,
                        )
                        .cast()
                        - relative_to,
                    Size::new(
                        i32::try_from(image.placement.width).expect("width out of range of i32"),
                        i32::try_from(image.placement.height).expect("height out of range of i32"),
                    ),
                ),
                color,
            );
            map(blit, &cached);

            glyphs
                .entry(glyph.cache_key.into())
                .or_insert_with(|| cached);
        }
    }
}

pub(crate) fn measure_text<Unit>(
    buffer: Option<&cosmic_text::Buffer>,
    kludgine: &mut Kludgine,
    queue: &wgpu::Queue,
    glyphs: &mut AHashMap<PixelAlignedCacheKey, CachedGlyphHandle>,
) -> MeasuredText<Unit>
where
    Unit: ScreenScale<Px = Px, Lp = Lp> + Sub<Output = Unit> + Copy + Debug,
{
    // TODO the returned type should be able to be drawn, so that we don't have to call update_scratch_buffer again.
    let line_height = dbg!(Unit::from_lp(kludgine.text.line_height, kludgine.scale));
    let mut min = Point::new(Px::MAX, Px::MAX);
    let mut max = Point::new(Px::MIN, Px::MIN);
    map_each_glyph(
        buffer,
        Color::WHITE,
        TextOrigin::TopLeft,
        kludgine,
        queue,
        glyphs,
        |blit, _cached| {
            min = min.min(blit.top_left().location);
            max = max.max(blit.bottom_right().location);
        },
    );

    MeasuredText {
        ascent: line_height - dbg!(Unit::from_px(min.y, kludgine.scale)),
        descent: line_height - dbg!(Unit::from_px(max.y, kludgine.scale)),
        left: Unit::from_px(min.x, kludgine.scale),
        width: Unit::from_px(max.x, kludgine.scale),
    }
}

/// Text that is ready to be rendered on the GPU.
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

/// Controls the origin of [`PreparedText`].
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub enum TextOrigin<Unit> {
    /// Render the text such that the top-left of the first line appears at the
    /// rendered location. When rotated, the text will rotate around the
    /// top-left of the text.
    #[default]
    TopLeft,
    /// Render the text such that the center of the extents of the rendered text
    /// appears at the rendered location. When rotated, the text will rotate
    /// around the geometric center of the rendered text.
    Center,
    /// Render the text such that the leftmost pixel of the baseline of the
    /// first line of text appears at the rendered location. When rotated, the
    /// text will rotate around this point.
    FirstBaseline,
    /// Render the text such that the text is offset by a custom amount. When
    /// rotated, the text will rotate around this point.
    Custom(Point<Unit>),
}

impl<Unit> ScreenScale for TextOrigin<Unit>
where
    Unit: ScreenScale<Px = Px, Lp = Lp>,
{
    type Lp = TextOrigin<Unit::Lp>;
    type Px = TextOrigin<Unit::Px>;

    fn into_px(self, scale: Fraction) -> Self::Px {
        match self {
            TextOrigin::TopLeft => TextOrigin::TopLeft,
            TextOrigin::Center => TextOrigin::Center,
            TextOrigin::FirstBaseline => TextOrigin::FirstBaseline,
            TextOrigin::Custom(pt) => TextOrigin::Custom(pt.into_px(scale)),
        }
    }

    fn from_px(px: Self::Px, scale: Fraction) -> Self {
        match px {
            TextOrigin::TopLeft => TextOrigin::TopLeft,
            TextOrigin::Center => TextOrigin::Center,
            TextOrigin::FirstBaseline => TextOrigin::FirstBaseline,
            TextOrigin::Custom(pt) => TextOrigin::Custom(Point::from_px(pt, scale)),
        }
    }

    fn into_lp(self, scale: Fraction) -> Self::Lp {
        match self {
            TextOrigin::TopLeft => TextOrigin::TopLeft,
            TextOrigin::Center => TextOrigin::Center,
            TextOrigin::FirstBaseline => TextOrigin::FirstBaseline,
            TextOrigin::Custom(pt) => TextOrigin::Custom(pt.into_lp(scale)),
        }
    }

    fn from_lp(dips: Self::Lp, scale: Fraction) -> Self {
        match dips {
            TextOrigin::TopLeft => TextOrigin::TopLeft,
            TextOrigin::Center => TextOrigin::Center,
            TextOrigin::FirstBaseline => TextOrigin::FirstBaseline,
            TextOrigin::Custom(pt) => TextOrigin::Custom(Point::from_lp(pt, scale)),
        }
    }
}

/// The dimensions of a measured text block.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MeasuredText<Unit> {
    /// The measurement above the baseline of the text.
    pub ascent: Unit,
    /// The measurement below the baseline of the text.
    pub descent: Unit,
    /// The measurement to the leftmost pixel of the text.
    pub left: Unit,
    /// The width of the measured text.
    pub width: Unit,
}
