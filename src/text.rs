use std::array;
use std::collections::{hash_map, HashMap};
use std::fmt::{self, Debug};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use cosmic_text::{Align, Attrs, AttrsOwned, LayoutGlyph, SwashContent};
use figures::units::{Lp, Px, UPx};
use figures::{
    FloatConversion, Fraction, IntoSigned, Point, Rect, Round, ScreenScale, Size, UPx2D, Zero,
};
use intentional::Cast;
use smallvec::SmallVec;

use crate::buffer::Buffer;
use crate::pipeline::PreparedCommand;
use crate::sealed::{ShapeSource, TextureSource};
use crate::{
    Assert, CanRenderTo, CollectedTexture, Color, DefaultHasher, DrawableSource, Graphics,
    Kludgine, PreparedGraphic, ProtoGraphics, TextureBlit, TextureCollection, VertexCollection,
};

impl Kludgine {
    /// Returns a mutable reference to the [`cosmic_text::FontSystem`] used when
    /// rendering text.
    pub fn font_system(&mut self) -> &mut cosmic_text::FontSystem {
        &mut self.text.fonts
    }

    /// Rebuilds the font system, invalidating font database caches.
    ///
    /// This function can be invoked after loading fonts into the font database
    /// to ensure that all future text rendering considers the newly loaded
    /// fonts.
    pub fn rebuild_font_system(&mut self) {
        let existing_system = std::mem::replace(
            &mut self.text.fonts,
            cosmic_text::FontSystem::new_with_fonts([]),
        );
        let (locale, db) = existing_system.into_locale_and_db();
        self.text.fonts = cosmic_text::FontSystem::new_with_locale_and_db(locale, db);
    }

    pub(crate) fn update_scratch_buffer(
        &mut self,
        text: &str,
        width: Option<Px>,
        align: Option<Align>,
    ) {
        self.text
            .update_scratch_buffer(text, self.effective_scale, width, align);
    }

    /// Sets the font size.
    pub fn set_font_size(&mut self, size: impl figures::ScreenScale<Lp = figures::units::Lp>) {
        self.text.set_font_size(
            figures::ScreenScale::into_lp(size, self.effective_scale),
            self.effective_scale,
        );
    }

    /// Returns the current font size.
    pub fn font_size(&self) -> figures::units::Lp {
        self.text.font_size
    }

    /// Sets the line height for multi-line layout.
    pub fn set_line_height(&mut self, size: impl figures::ScreenScale<Lp = figures::units::Lp>) {
        self.text.set_line_height(
            figures::ScreenScale::into_lp(size, self.effective_scale),
            self.effective_scale,
        );
    }

    /// Returns the current line height.
    pub fn line_height(&self) -> figures::units::Lp {
        self.text.line_height
    }

    /// Sets the current font family.
    pub fn set_font_family(&mut self, family: cosmic_text::FamilyOwned) {
        self.text.attrs.family_owned = family;
    }

    /// Returns the current font family.
    pub fn font_family(&self) -> cosmic_text::Family<'_> {
        self.text.attrs.family_owned.as_family()
    }

    /// Sets the current font style.
    pub fn set_font_style(&mut self, style: cosmic_text::Style) {
        self.text.attrs.style = style;
    }

    /// Returns the current font style.
    pub fn font_style(&self) -> cosmic_text::Style {
        self.text.attrs.style
    }

    /// Sets the current font weight.
    pub fn set_font_weight(&mut self, weight: cosmic_text::Weight) {
        self.text.attrs.weight = weight;
    }

    /// Returns the current font weight.
    pub fn font_weight(&self) -> cosmic_text::Weight {
        self.text.attrs.weight
    }

    /// Sets the current text stretching.
    pub fn set_text_stretch(&mut self, width: cosmic_text::Stretch) {
        self.text.attrs.stretch = width;
    }

    /// Returns the current text stretch.
    pub fn text_stretch(&self) -> cosmic_text::Stretch {
        self.text.attrs.stretch
    }

    /// Returns the current text attributes.
    pub fn text_attrs(&self) -> cosmic_text::Attrs<'_> {
        self.text.attrs.as_attrs()
    }

    /// Sets the current text attributes.
    pub fn set_text_attributes(&mut self, attrs: Attrs<'_>) {
        self.text.attrs = AttrsOwned::new(attrs);
    }

    /// Resets all of the text related properties to their default settings.
    pub fn reset_text_attributes(&mut self) {
        self.set_text_attributes(Attrs::new());
        self.text.font_size = DEFAULT_FONT_SIZE;
        self.text.line_height = DEFAULT_LINE_SIZE;
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

impl Debug for TextSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextSystem")
            .field("font_size", &self.font_size)
            .field("line_height", &self.line_height)
            .field("attrs", &self.attrs)
            .field("glyphs", &self.glyphs)
            .finish_non_exhaustive()
    }
}

const DEFAULT_FONT_SIZE: Lp = Lp::points(12);
const DEFAULT_LINE_SIZE: Lp = Lp::points(16);

impl TextSystem {
    pub(crate) fn new(graphics: &ProtoGraphics<'_>) -> Self {
        let fonts = cosmic_text::FontSystem::new();

        Self {
            alpha_text_atlas: TextureCollection::new_generic(
                Size::new(512, 512).cast(),
                wgpu::TextureFormat::R8Unorm,
                wgpu::FilterMode::Linear,
                graphics,
            ),
            color_text_atlas: TextureCollection::new_generic(
                Size::new(512, 512).cast(),
                wgpu::TextureFormat::Rgba8UnormSrgb,
                wgpu::FilterMode::Linear,
                graphics,
            ),
            swash_cache: cosmic_text::SwashCache::new(),
            scratch: None,
            fonts,
            font_size: DEFAULT_FONT_SIZE,
            line_height: DEFAULT_LINE_SIZE,
            glyphs: GlyphCache::default(),
            attrs: AttrsOwned::new(Attrs::new()),
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

    pub fn update_scratch_buffer(
        &mut self,
        text: &str,
        scale: Fraction,
        width: Option<Px>,
        align: Option<Align>,
    ) {
        if self.scratch.is_none() {
            let metrics = self.metrics(scale);
            let buffer = cosmic_text::Buffer::new(&mut self.fonts, metrics);
            self.scratch = Some(buffer);
        }

        let scratch = self.scratch.as_mut().expect("initialized above");
        scratch.set_text(
            &mut self.fonts,
            text,
            self.attrs.as_attrs(),
            cosmic_text::Shaping::Advanced, // TODO maybe this should be configurable?
        );
        scratch.set_size(&mut self.fonts, width.map(Cast::cast), None);
        for line in &mut scratch.lines {
            line.set_align(align);
        }
        scratch.shape_until_scroll(&mut self.fonts, false);
    }
}

#[derive(Debug, Default, Clone)]
struct GlyphCache {
    glyphs: Arc<Mutex<HashMap<cosmic_text::CacheKey, CachedGlyph, DefaultHasher>>>,
}

impl GlyphCache {
    fn get_or_insert(
        &self,
        key: cosmic_text::CacheKey,
        insert_fn: impl FnOnce() -> Option<(CollectedTexture, bool)>,
    ) -> Option<CachedGlyphHandle> {
        let mut data = self.glyphs.lock().unwrap_or_else(PoisonError::into_inner);
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
            cache: Arc::downgrade(&self.glyphs),
            texture: cached.texture.clone(),
        })
    }

    fn clear_unused(&mut self) {
        let mut data = self.glyphs.lock().unwrap_or_else(PoisonError::into_inner);
        data.retain(|_, glyph| glyph.ref_count > 0);
    }
}

#[derive(Debug)]
struct CachedGlyph {
    texture: CollectedTexture,
    is_mask: bool,
    ref_count: usize,
}

pub(crate) struct CachedGlyphHandle {
    key: cosmic_text::CacheKey,
    pub is_mask: bool,
    cache: Weak<Mutex<HashMap<cosmic_text::CacheKey, CachedGlyph, DefaultHasher>>>,
    pub texture: CollectedTexture,
}

impl Debug for CachedGlyphHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedGlyphHandle")
            .field("key", &self.key)
            .field("is_mask", &self.is_mask)
            .finish_non_exhaustive()
    }
}

impl Clone for CachedGlyphHandle {
    fn clone(&self) -> Self {
        if let Some(glyphs) = self.cache.upgrade() {
            let mut data = glyphs.lock().unwrap_or_else(PoisonError::into_inner);
            let cached = data.get_mut(&self.key).expect("cached glyph missing");
            cached.ref_count += 1;
            drop(data);
        }

        Self {
            key: self.key,
            is_mask: self.is_mask,
            cache: self.cache.clone(),
            texture: self.texture.clone(),
        }
    }
}

impl Drop for CachedGlyphHandle {
    fn drop(&mut self) {
        if let Some(glyphs) = self.cache.upgrade() {
            let mut data = glyphs.lock().unwrap_or_else(PoisonError::into_inner);
            let cached = data.get_mut(&self.key).expect("cached glyph missing");
            cached.ref_count -= 1;
        }
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
        let mut glyphs = HashMap::default();
        let mut vertices = VertexCollection::default();
        let mut indices = Vec::new();
        let mut commands = SmallVec::<[PreparedCommand; 2]>::new();

        map_each_glyph(
            Some(buffer),
            default_color,
            origin,
            self.kludgine,
            self.device,
            self.queue,
            &mut glyphs,
            |blit, _glyph, _is_first_line, _baseline, _line_w, kludgine| {
                if let GlyphBlit::Visible {
                    blit,
                    glyph: cached,
                } = blit
                {
                    let corners: [u32; 4] =
                        array::from_fn(|index| vertices.get_or_insert(blit.verticies[index]));
                    let start_index = u32::try_from(indices.len()).assert("too many drawn indices");
                    for &index in blit.indices() {
                        indices
                            .push(corners[usize::try_from(index).assert("too many drawn indices")]);
                    }
                    let end_index = u32::try_from(indices.len()).assert("too many drawn indices");
                    match commands.last_mut() {
                        Some(last_command) if last_command.is_mask == cached.is_mask => {
                            // The last command was from the same texture source, we can stend the previous range to the new end.
                            last_command.indices.end = end_index;
                        }
                        _ => {
                            commands.push(PreparedCommand {
                                indices: start_index..end_index,
                                is_mask: cached.is_mask,
                                binding: Some(cached.texture.bind_group(&ProtoGraphics::new(
                                    self.device,
                                    self.queue,
                                    kludgine,
                                ))),
                            });
                        }
                    }
                }
            },
        );

        PreparedText {
            graphic: PreparedGraphic {
                vertices: Buffer::new(&vertices.vertices, wgpu::BufferUsages::VERTEX, self.device),
                indices: Buffer::new(&indices, wgpu::BufferUsages::INDEX, self.device),
                commands,
            },
            _glyphs: glyphs,
        }
    }
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn map_each_glyph(
    buffer: Option<&cosmic_text::Buffer>,
    default_color: Color,
    origin: TextOrigin<Px>,
    kludgine: &mut Kludgine,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    glyphs: &mut HashMap<cosmic_text::CacheKey, CachedGlyphHandle, DefaultHasher>,
    mut map: impl for<'a> FnMut(GlyphBlit, &'a LayoutGlyph, usize, Px, Px, &'a Kludgine),
) {
    let metrics = buffer
        .unwrap_or_else(|| kludgine.text.scratch.as_ref().expect("no buffer"))
        .metrics();

    let line_height_offset = Point::new(Px::ZERO, Px::from(metrics.line_height));
    let relative_to = match origin {
        TextOrigin::Custom(point) => point,
        TextOrigin::TopLeft => Point::default(),
        TextOrigin::Center => {
            let measured =
                measure_text::<Px, false>(buffer, default_color, kludgine, device, queue, glyphs);
            (Point::from(measured.size) / 2).round()
        }
        TextOrigin::FirstBaseline => line_height_offset.cast(),
    } + line_height_offset;

    let buffer = buffer.unwrap_or_else(|| kludgine.text.scratch.as_ref().expect("no buffer"));
    for run in buffer.layout_runs() {
        let run_origin = Point::new(Px::ZERO, Px::from(run.line_y)) - relative_to;
        for glyph in run.glyphs {
            let physical =
                glyph.physical((run_origin.x.into_float(), run_origin.y.into_float()), 1.);
            let Some(image) = kludgine
                .text
                .swash_cache
                .get_image(&mut kludgine.text.fonts, physical.cache_key)
            else {
                continue;
            };
            let invisible = image.placement.width == 0 || image.placement.height == 0;

            let mut color = glyph.color_opt.map_or(default_color, Color::from);

            let cached = if invisible {
                None
            } else {
                kludgine
                    .text
                    .glyphs
                    .get_or_insert(physical.cache_key, || match image.content {
                        SwashContent::Mask => Some((
                            kludgine.text.alpha_text_atlas.push_texture_generic(
                                &image.data,
                                wgpu::ImageDataLayout {
                                    offset: 0,
                                    bytes_per_row: Some(image.placement.width),
                                    rows_per_image: None,
                                },
                                Size::upx(image.placement.width, image.placement.height),
                                &ProtoGraphics {
                                    id: kludgine.id,
                                    device,
                                    queue,
                                    binding_layout: &kludgine.binding_layout,
                                    linear_sampler: &kludgine.linear_sampler,
                                    nearest_sampler: &kludgine.nearest_sampler,
                                    uniforms: &kludgine.uniforms.wgpu,
                                    multisample: kludgine.multisample,
                                },
                            ),
                            true,
                        )),
                        SwashContent::Color => {
                            // Set the color to full white to avoid mixing.
                            color = Color::WHITE;
                            Some((
                                kludgine.text.color_text_atlas.push_texture_generic(
                                    &image.data,
                                    wgpu::ImageDataLayout {
                                        offset: 0,
                                        bytes_per_row: Some(image.placement.width * 4),
                                        rows_per_image: None,
                                    },
                                    Size::upx(image.placement.width, image.placement.height),
                                    &ProtoGraphics {
                                        id: kludgine.id,
                                        device,
                                        queue,
                                        binding_layout: &kludgine.binding_layout,
                                        linear_sampler: &kludgine.linear_sampler,
                                        nearest_sampler: &kludgine.nearest_sampler,
                                        uniforms: &kludgine.uniforms.wgpu,
                                        multisample: kludgine.multisample,
                                    },
                                ),
                                false,
                            ))
                        }
                        SwashContent::SubpixelMask => None,
                    })
            };

            let blit = if let Some(cached) = cached {
                glyphs
                    .entry(physical.cache_key)
                    .or_insert_with(|| cached.clone());

                GlyphBlit::Visible {
                    blit: TextureBlit::new(
                        cached.texture.region,
                        Rect::new(
                            (Point::new(physical.x, physical.y)).cast::<Px>()
                                + Point::new(
                                    Px::new(image.placement.left),
                                    Px::from(metrics.line_height) - image.placement.top,
                                ),
                            Size::new(
                                UPx::new(image.placement.width),
                                UPx::new(image.placement.height),
                            )
                            .into_signed(),
                        ),
                        color,
                    ),
                    glyph: cached.clone(),
                }
            } else {
                GlyphBlit::Invisible {
                    location: Point::new(physical.x, physical.y).cast::<Px>(),
                    width: glyph.w.cast(),
                }
            };
            map(
                blit,
                glyph,
                (run.line_top / metrics.line_height).round().cast::<usize>(),
                Px::from(run.line_y),
                Px::from(run.line_w.ceil()),
                kludgine,
            );
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum GlyphBlit {
    Invisible {
        location: Point<Px>,
        width: Px,
    },
    Visible {
        blit: TextureBlit<Px>,
        glyph: CachedGlyphHandle,
    },
}

impl GlyphBlit {
    pub fn top_left(&self) -> Point<Px> {
        match self {
            GlyphBlit::Invisible { location, .. } => *location,
            GlyphBlit::Visible { blit, .. } => blit.top_left().location,
        }
    }

    pub fn bottom_right(&self, bottom: Px) -> Point<Px> {
        match self {
            GlyphBlit::Invisible { location, width } => Point::new(location.x + *width, bottom),
            GlyphBlit::Visible { blit, .. } => blit.bottom_right().location,
        }
    }
}

impl CanRenderTo for GlyphBlit {
    fn can_render_to(&self, kludgine: &Kludgine) -> bool {
        match self {
            GlyphBlit::Invisible { .. } => true,
            GlyphBlit::Visible { glyph, .. } => glyph.texture.can_render_to(kludgine),
        }
    }
}

pub(crate) fn measure_text<Unit, const COLLECT_GLYPHS: bool>(
    buffer: Option<&cosmic_text::Buffer>,
    color: Color,
    kludgine: &mut Kludgine,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    glyphs: &mut HashMap<cosmic_text::CacheKey, CachedGlyphHandle, DefaultHasher>,
) -> MeasuredText<Unit>
where
    Unit: figures::ScreenUnit,
{
    // TODO the returned type should be able to be drawn, so that we don't have to call update_scratch_buffer again.
    let line_height = Unit::from_lp(kludgine.text.line_height, kludgine.effective_scale);
    let mut min = Point::new(Px::MAX, Px::MAX);
    let mut last_baseline = Px::MIN;
    let mut max = Point::new(Px::MIN, Px::MIN);
    let mut ascent = Px::ZERO;
    let mut descent = Px::ZERO;
    let mut first_baseline = Px::ZERO;
    let mut measured_glyphs = Vec::new();
    map_each_glyph(
        buffer,
        color,
        TextOrigin::TopLeft,
        kludgine,
        device,
        queue,
        glyphs,
        |blit, glyph, line_index, baseline, line_width, _kludgine| {
            last_baseline = last_baseline.max(baseline);
            min = min.min(blit.top_left());
            max.x = max.x.max(line_width);
            max.y = max.y.max(blit.bottom_right(baseline).y);
            if line_index == 0 {
                first_baseline = baseline;
                ascent = ascent.max(baseline - blit.top_left().y);
                descent = descent.min(baseline - blit.bottom_right(baseline).y);
            }
            if COLLECT_GLYPHS {
                measured_glyphs.push(MeasuredGlyph {
                    blit,
                    info: GlyphInfo::new(glyph, line_index, line_width),
                });
            }
        },
    );

    if min == Point::new(Px::MAX, Px::MAX) {
        MeasuredText {
            ascent: Unit::default(),
            descent: Unit::default(),
            left: Unit::default(),
            line_height,
            size: Size::new(Unit::default(), line_height),
            glyphs: Vec::new(),
        }
    } else {
        MeasuredText {
            ascent: Unit::from_px(ascent, kludgine.effective_scale),
            descent: Unit::from_px(descent, kludgine.effective_scale),
            left: Unit::from_px(min.x, kludgine.effective_scale),
            size: Size {
                width: Unit::from_px(max.x, kludgine.effective_scale),
                height: Unit::from_px(max.y.max(last_baseline), kludgine.effective_scale)
                    .max(line_height),
            },
            line_height: Unit::from_px(first_baseline, kludgine.effective_scale),
            glyphs: measured_glyphs,
        }
    }
}

/// Text that is ready to be rendered on the GPU.
pub struct PreparedText {
    graphic: PreparedGraphic<Px>,
    _glyphs: HashMap<cosmic_text::CacheKey, CachedGlyphHandle, DefaultHasher>,
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
    Unit: ScreenScale<Px = Px, Lp = Lp, UPx = UPx>,
{
    type Lp = TextOrigin<Unit::Lp>;
    type Px = TextOrigin<Unit::Px>;
    type UPx = TextOrigin<Unit::UPx>;

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

    fn into_upx(self, scale: Fraction) -> Self::UPx {
        match self {
            TextOrigin::TopLeft => TextOrigin::TopLeft,
            TextOrigin::Center => TextOrigin::Center,
            TextOrigin::FirstBaseline => TextOrigin::FirstBaseline,
            TextOrigin::Custom(pt) => TextOrigin::Custom(pt.into_upx(scale)),
        }
    }

    fn from_upx(px: Self::UPx, scale: Fraction) -> Self {
        match px {
            TextOrigin::TopLeft => TextOrigin::TopLeft,
            TextOrigin::Center => TextOrigin::Center,
            TextOrigin::FirstBaseline => TextOrigin::FirstBaseline,
            TextOrigin::Custom(px) => TextOrigin::Custom(Point::from_upx(px, scale)),
        }
    }
}

/// The dimensions of a measured text block.
#[derive(Debug, Clone)]
pub struct MeasuredText<Unit> {
    /// The measurement above the baseline of the text.
    pub ascent: Unit,
    /// The measurement below the baseline of the text.
    pub descent: Unit,
    /// The measurement to the leftmost pixel of the text.
    pub left: Unit,
    /// The measurement above the baseline of the text.
    pub line_height: Unit,
    /// The total size of the measured text, encompassing all lines.
    pub size: Size<Unit>,
    /// The individual glyhs that were laid out.
    pub glyphs: Vec<MeasuredGlyph>,
}

impl<Unit> CanRenderTo for MeasuredText<Unit> {
    fn can_render_to(&self, kludgine: &Kludgine) -> bool {
        self.glyphs
            .first()
            .map_or(true, |glyph| glyph.can_render_to(kludgine))
    }
}

impl<Unit> DrawableSource for MeasuredText<Unit> {}

/// Instructions for drawing a laid out glyph.
#[derive(Clone)]
pub struct MeasuredGlyph {
    pub(crate) blit: GlyphBlit,
    /// Information about what glyph this is.
    pub info: GlyphInfo,
}

impl Debug for MeasuredGlyph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MeasuredGlyph")
            .field("blit", &self.blit)
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

impl MeasuredGlyph {
    /// Returns the destination rectangle for this glyph.
    #[must_use]
    pub fn rect(&self) -> Rect<Px> {
        let top_left = self.blit.top_left();
        Rect::from_extents(top_left, self.blit.bottom_right(top_left.y))
    }

    /// Returns true if this measurement is for a visible glyph, as opposed to
    /// whitespace or padding.
    #[must_use]
    pub const fn visible(&self) -> bool {
        matches!(self.blit, GlyphBlit::Visible { .. })
    }
}

impl CanRenderTo for MeasuredGlyph {
    fn can_render_to(&self, kludgine: &Kludgine) -> bool {
        self.blit.can_render_to(kludgine)
    }
}

/// Information about a glyph in a [`MeasuredText`].
#[derive(Debug, Clone, Copy)]
pub struct GlyphInfo {
    /// Start index of cluster in original line
    pub start: usize,
    /// End index of cluster in original line
    pub end: usize,
    /// The line index this glyph is visually laid out on.
    pub line: usize,
    /// The width of the line this glyph is on.
    ///
    /// Because whitespace does not have glyphs, this width may be useful in
    /// measuring whitespace at the end of a line.
    pub line_width: Px,
    /// Unicode `BiDi` embedding level, character is left-to-right if `level` is divisible by 2
    pub level: unicode_bidi::Level,
    /// Custom metadata set in [`cosmic_text::Attrs`].
    pub metadata: usize,
}

impl GlyphInfo {
    fn new(glyph: &LayoutGlyph, line: usize, line_width: Px) -> Self {
        Self {
            start: glyph.start,
            end: glyph.end,
            line,
            line_width,
            metadata: glyph.metadata,
            level: glyph.level,
        }
    }
}

/// A text drawing command.
#[derive(Clone, Copy, Debug)]
pub struct Text<'a, Unit> {
    /// The text to be drawn.
    pub(crate) text: &'a str,
    /// The color to draw the text using.
    pub(crate) color: Color,
    /// The origin to draw the text around.
    pub(crate) origin: TextOrigin<Unit>,
    /// The width to wrap the text at. If `None`, no wrapping is performed.
    pub(crate) wrap_at: Option<Unit>,
    pub(crate) align: Option<Align>,
}

impl<'a, Unit> Text<'a, Unit> {
    /// Returns a text command that draws `text` with `color`.
    #[must_use]
    pub const fn new(text: &'a str, color: Color) -> Self {
        Self {
            text,
            color,
            origin: TextOrigin::TopLeft,
            wrap_at: None,
            align: None,
        }
    }

    /// Sets the origin for the text drawing operation and returns self.
    #[must_use]
    pub fn origin(mut self, origin: TextOrigin<Unit>) -> Self {
        self.origin = origin;
        self
    }

    /// Sets the width to wrap text at and returns self.
    #[must_use]
    pub fn wrap_at(mut self, width: Unit) -> Self {
        self.wrap_at = Some(width);
        self
    }

    /// Aligns this text using the specified alignment within the specified
    /// layout width.
    #[must_use]
    pub fn align(mut self, align: Align, width: Unit) -> Self {
        self.wrap_at = Some(width);
        self.align = Some(align);
        self
    }
}

impl<'a, Unit> From<&'a str> for Text<'a, Unit> {
    fn from(value: &'a str) -> Self {
        Self::new(value, Color::WHITE)
    }
}

impl<'a, Unit> From<&'a String> for Text<'a, Unit> {
    fn from(value: &'a String) -> Self {
        Self::new(value, Color::WHITE)
    }
}

impl<'a, Unit> DrawableSource for Text<'a, Unit> {}
