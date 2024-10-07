use std::collections::{hash_map, HashMap};
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::iter::IntoIterator;
use std::ops::{Deref, Div};
use std::sync::Arc;
use std::time::Duration;

use figures::units::UPx;
use figures::{Point, Rect, Size};
use intentional::{Assert, Cast};
use justjson::Value;

use crate::pipeline::Vertex;
use crate::sealed::{self, TextureSource as _};
use crate::{
    CanRenderTo, CollectedTexture, Graphics, Kludgine, PreparedGraphic, ShareableTexture,
    SharedTexture, TextureRegion, TextureSource,
};

/// Includes an [Aseprite](https://www.aseprite.org/) sprite sheet and Json
/// export. For more information, see [`Sprite::load_aseprite_json`]. This macro
/// will append ".png" and ".json" to the path provided and include both files
/// in your binary.
#[macro_export]
macro_rules! include_aseprite_sprite {
    ($path:expr) => {{
        $crate::include_texture!(concat!($path, ".png"))
            .map_err($crate::sprite::SpriteParseError::from)
            .and_then(|texture| {
                $crate::sprite::Sprite::load_aseprite_json(
                    include_str!(concat!($path, ".json")),
                    texture,
                )
            })
    }};
}

/// The animation mode of the sprite.
#[derive(Debug, Clone)]
pub enum AnimationMode {
    /// Iterate frames in order. When at the end, reset to the start.
    Forward,
    /// Iterate frames in reverse order. When at the start, reset to the end.
    Reverse,
    /// Iterate frames starting at the beginning and continuously iterating
    /// forwards and backwards across the frames, changing direction whenever
    /// the start or end are reached.
    PingPong,
}

impl AnimationMode {
    const fn default_direction(&self) -> AnimationDirection {
        match self {
            AnimationMode::Forward | AnimationMode::PingPong => AnimationDirection::Forward,
            AnimationMode::Reverse => AnimationDirection::Reverse,
        }
    }
}

#[derive(Debug, Clone)]
enum AnimationDirection {
    Forward,
    Reverse,
}

/// An error occurred parsing a [`Sprite`].
#[derive(Debug)]
pub enum SpriteParseError {
    /// The `meta` field is missing or invalid.
    Meta,
    /// The size information is missing.
    SizeMissing,
    /// The size does not match the provided texture.
    SizeMismatch,
    /// An error parsing a frame tag (animation).
    FrameTag {
        /// The name of the frame tag.
        name: String,
        /// The error that occurred.
        error: FrameTagError,
    },
    /// An error occurred parsing a frame.
    Frame {
        /// The object key for the frame.
        key: String,
        /// The error that occurred.
        error: FrameParseError,
    },
    /// Invalid JSON.
    Json(justjson::Error),
    /// An image parsing error.
    #[cfg(feature = "image")]
    Image(image::ImageError),
}

impl SpriteParseError {
    fn frame(key: &impl Display, error: FrameParseError) -> Self {
        Self::Frame {
            key: key.to_string(),
            error,
        }
    }

    fn frame_tag(name: &impl Display, error: FrameTagError) -> Self {
        Self::FrameTag {
            name: name.to_string(),
            error,
        }
    }
}

#[cfg(feature = "image")]
impl From<image::ImageError> for SpriteParseError {
    fn from(value: image::ImageError) -> Self {
        Self::Image(value)
    }
}

/// An error parsing a single frame in a sprite animation.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum FrameParseError {
    /// The frame number was not able to be parsed as a number.
    NotNumeric,
    /// The duration is invalid or missing.
    Duration,
    /// The data is missing the `frame` field, which contains the region in the
    /// texture to use for this frame.
    MissingRegion,
    /// The `frame.x` value is missing or invalid.
    X,
    /// The `frame.y` value is missing or invalid.
    Y,
    /// The `frame.w` value is missing or invalid.
    Width,
    /// The `frame.h` value is missing or invalid.
    Height,
}

/// An error parsing a `frameTags` entry.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum FrameTagError {
    /// The direction field is missing.
    DirectionMissing,
    /// The direction is not a recognized value.
    DirectionUnknown,
    /// The from field is missing or invalid.
    From,
    /// The to field is missing or invalid.
    To,
    /// The frame could not be found.
    InvalidFrame,
}

impl From<justjson::Error> for SpriteParseError {
    fn from(error: justjson::Error) -> Self {
        Self::Json(error)
    }
}

/// A [`Sprite`]'s tag did not correspond to an animation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidSpriteTag;

/// A sprite is a renderable graphic with optional animations.
///
/// Cloning a sprite is cheap. When cloning, the animations will be shared
/// between all clones of the sprite, but each sprite will track its current
/// frame/tag independently.
#[derive(Debug, Clone)]
pub struct Sprite {
    /// The animations that form this sprite.
    pub animations: SpriteAnimations,
    elapsed_since_frame_change: Duration,
    current_tag: Option<String>,
    current_frame: usize,
    current_animation_direction: AnimationDirection,
}

impl From<SpriteAnimations> for Sprite {
    fn from(animations: SpriteAnimations) -> Self {
        Self::new(animations)
    }
}

impl Sprite {
    /// Returns a new sprite with `animations`.
    #[must_use]
    pub const fn new(animations: SpriteAnimations) -> Self {
        Self {
            animations,
            current_frame: 0,
            current_tag: None,
            elapsed_since_frame_change: Duration::from_millis(0),
            current_animation_direction: AnimationDirection::Forward,
        }
    }

    /// For merging multiple Sprites that have no tags within them
    #[must_use]
    pub fn merged<S: Into<String>, I: IntoIterator<Item = (S, Self)>>(source: I) -> Self {
        let mut combined = HashMap::new();
        for (name, sprite) in source {
            let Some(animation) = sprite.animations.animation_for(&Option::<&str>::None) else {
                continue;
            };
            combined.insert(Some(name.into()), animation.clone());
        }
        Self::new(SpriteAnimations::new(combined))
    }

    /// Creates an instance from a texture. This creates a `SpriteAnimation`
    /// with no tag and a single frame.
    #[must_use]
    pub fn single_frame(texture: SharedTexture) -> Self {
        let mut frames = HashMap::new();
        frames.insert(
            None,
            SpriteAnimation::new(vec![SpriteFrame::new(TextureRegion::from(texture))])
                .with_mode(AnimationMode::Forward),
        );
        let frames = SpriteAnimations::new(frames);

        Self::new(frames)
    }

    /// Loads [Aseprite](https://www.aseprite.org/) JSON export format, when
    /// using the correct settings.
    ///
    /// For the JSON data, use the Hash export option (default), and use either
    /// spaces or underscores (_) inbetween the fields in the name. Ensure
    /// `{frame}` is the last field in the name before the extension. E.g.,
    /// `{tag}_{frame}.{extension}`
    ///
    /// # Errors
    ///
    /// Returns an error when `raw_json` does not match the expected format.
    #[allow(clippy::too_many_lines)]
    pub fn load_aseprite_json(
        raw_json: &str,
        texture: impl Into<ShareableTexture>,
    ) -> Result<Self, SpriteParseError> {
        let texture = texture.into();
        let json = justjson::Value::from_json(raw_json)?;

        let Some(Value::Object(meta)) = json.get("meta") else {
            return Err(SpriteParseError::Meta);
        };

        let texture_size = texture.default_rect().size;
        let Some(size) = meta.get("size") else {
            return Err(SpriteParseError::SizeMissing);
        };

        if size["w"].as_u32() != Some(texture_size.width.get())
            || size["h"].as_u32() != Some(texture_size.height.get())
        {
            return Err(SpriteParseError::SizeMismatch);
        }

        let mut frames = HashMap::new();
        for frame in json["frames"]
            .as_object()
            .map(|frame| frame.iter())
            .into_iter()
            .flatten()
        {
            // Remove the extension, if present
            let key = frame.key.decode_if_needed();
            let name = key.split('.').next().assert_expected();
            // Split by _ or ' 'as per the documentation of this method.
            let name_parts = name.split(['_', ' ']).collect::<Vec<_>>();
            let frame_number = name_parts[name_parts.len() - 1]
                .parse::<usize>()
                .or_else(|_| {
                    if json["frames"]
                        .as_array()
                        .map_or(false, |frames| frames.len() == 1)
                    {
                        Ok(0)
                    } else {
                        Err(SpriteParseError::frame(
                            &frame.key,
                            FrameParseError::NotNumeric,
                        ))
                    }
                })?;

            let duration = match frame.value.get("duration").and_then(Value::as_u64) {
                Some(millis) => Duration::from_millis(millis),
                None => {
                    return Err(SpriteParseError::frame(
                        &frame.key,
                        FrameParseError::Duration,
                    ))
                }
            };

            let Some(rect) = frame.value.get("frame") else {
                return Err(SpriteParseError::frame(
                    &frame.key,
                    FrameParseError::MissingRegion,
                ));
            };

            let region = Rect::new(
                Point::new(
                    rect["x"]
                        .as_u32()
                        .ok_or_else(|| SpriteParseError::frame(&frame.key, FrameParseError::X))?,
                    rect["y"]
                        .as_u32()
                        .ok_or_else(|| SpriteParseError::frame(&frame.key, FrameParseError::Y))?,
                ),
                Size::new(
                    rect["w"].as_u32().ok_or_else(|| {
                        SpriteParseError::frame(&frame.key, FrameParseError::Width)
                    })?,
                    rect["h"].as_u32().ok_or_else(|| {
                        SpriteParseError::frame(&frame.key, FrameParseError::Height)
                    })?,
                ),
            )
            .cast();

            let source = SpriteSource::Region(TextureRegion {
                region,
                texture: texture.clone(),
            });

            frames.insert(
                frame_number,
                SpriteFrame {
                    duration: Some(duration),
                    source,
                },
            );
        }

        let mut animations = HashMap::new();
        for tag in meta
            .get("frameTags")
            .and_then(|tags| tags.as_array())
            .into_iter()
            .flatten()
        {
            let Some(name) = tag["name"].as_string() else {
                continue;
            };

            let Some(direction) = tag["direction"].as_string() else {
                return Err(SpriteParseError::frame_tag(
                    name,
                    FrameTagError::DirectionMissing,
                ));
            };
            let direction = if direction == "forward" {
                AnimationMode::Forward
            } else if direction == "reverse" {
                AnimationMode::Reverse
            } else if direction == "pingpong" {
                AnimationMode::PingPong
            } else {
                return Err(SpriteParseError::frame_tag(
                    name,
                    FrameTagError::DirectionUnknown,
                ));
            };

            let start_frame = tag["from"]
                .as_usize()
                .ok_or_else(|| SpriteParseError::frame_tag(name, FrameTagError::From))?;
            let end_frame = tag["to"]
                .as_usize()
                .ok_or_else(|| SpriteParseError::frame_tag(name, FrameTagError::To))?;
            let mut animation_frames = Vec::new();
            for i in start_frame..=end_frame {
                let frame = frames.get(&i).ok_or_else(|| {
                    SpriteParseError::frame_tag(name, FrameTagError::InvalidFrame)
                })?;
                animation_frames.push(frame.clone());
            }

            animations.insert(
                Some(name.to_string()),
                SpriteAnimation::new(animation_frames).with_mode(direction),
            );
        }

        let mut frames: Vec<_> = frames.into_iter().collect();
        frames.sort_by(|a, b| a.0.cmp(&b.0));

        animations.insert(
            None,
            SpriteAnimation::new(frames.iter().map(|(_, f)| f.clone()).collect())
                .with_mode(AnimationMode::Forward),
        );

        Ok(Self::new(SpriteAnimations::new(animations)))
    }

    /// Sets the current tag for the animation. If the tag currently matches,
    /// nothing will happen. If it is a new tag, the current frame and animation
    /// direction will be switched to the values from the new tag.
    ///
    /// # Errors
    ///
    /// Returns an error if `tag` is not a valid animation tag.
    pub fn set_current_tag<S: Into<String>>(
        &mut self,
        tag: Option<S>,
    ) -> Result<(), InvalidSpriteTag> {
        let new_tag = tag.map(Into::into);
        if self.current_tag != new_tag {
            self.current_animation_direction = {
                let animation = self
                    .animations
                    .animations
                    .get(&new_tag)
                    .ok_or(InvalidSpriteTag)?;
                animation.mode.default_direction()
            };
            self.current_frame = 0;
            self.current_tag = new_tag;
        }

        Ok(())
    }

    /// Returns the current tag.
    #[must_use]
    pub fn current_tag(&self) -> Option<&'_ str> {
        self.current_tag.as_deref()
    }

    /// Gets the current frame after advancing the animation for `elapsed`
    /// duration. If you need to invoke this multiple times in a single frame,
    /// pass `None` on subsequent calls. In general, you should clone sprites
    /// rather than reuse them. Kludgine ensures that your texture and animation
    /// data will be shared and not cloned.
    ///
    /// # Errors
    ///
    /// Returns an error the current animation tag does not match any defined
    /// animation.
    pub fn get_frame(
        &mut self,
        elapsed: Option<Duration>,
    ) -> Result<SpriteSource, InvalidSpriteTag> {
        if let Some(elapsed) = elapsed {
            self.elapsed_since_frame_change += elapsed;

            let current_frame_duration = self.with_current_frame(|frame| frame.duration)?;
            if let Some(frame_duration) = current_frame_duration {
                if self.elapsed_since_frame_change > frame_duration {
                    self.elapsed_since_frame_change = Duration::from_nanos(
                        (self.elapsed_since_frame_change.as_nanos() % frame_duration.as_nanos())
                            .cast(),
                    );
                    self.advance_frame()?;
                }
            }
        }

        self.current_frame()
    }

    /// Retrieve the current animation frame, if set and valid.
    ///
    /// # Errors
    ///
    /// Returns an error the current animation tag does not match any defined
    /// animation.
    #[inline]
    pub fn current_frame(&self) -> Result<SpriteSource, InvalidSpriteTag> {
        self.with_current_frame(|frame| frame.source.clone())
    }

    /// Returns the amount of time remaining until the next frame is due to be
    /// shown for this sprite. Can be used to calculate redraws more efficiently
    /// if you're not rendering at a constant framerate.
    ///
    /// # Errors
    ///
    /// Returns an error the current animation tag does not match any defined
    /// animation.
    pub fn remaining_frame_duration(&self) -> Result<Option<Duration>, InvalidSpriteTag> {
        let duration = self
            .with_current_frame(|frame| frame.duration)?
            .map(|frame_duration| {
                frame_duration
                    .checked_sub(self.elapsed_since_frame_change)
                    .unwrap_or_default()
            });

        Ok(duration)
    }

    fn advance_frame(&mut self) -> Result<(), InvalidSpriteTag> {
        self.current_frame = self.next_frame()?;
        Ok(())
    }

    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    fn next_frame(&mut self) -> Result<usize, InvalidSpriteTag> {
        let starting_frame = self.current_frame.cast::<i32>();
        let animation = self
            .animations
            .animations
            .get(&self.current_tag)
            .ok_or(InvalidSpriteTag)?;

        let next_frame = match self.current_animation_direction {
            AnimationDirection::Forward => starting_frame + 1,
            AnimationDirection::Reverse => starting_frame - 1,
        };

        Ok(if next_frame < 0 {
            match animation.mode {
                AnimationMode::Forward => unreachable!(),
                AnimationMode::Reverse => {
                    // Cycle back to the last frame
                    animation.frames.len() - 1
                }
                AnimationMode::PingPong => {
                    self.current_animation_direction = AnimationDirection::Forward;
                    1
                }
            }
        } else if next_frame as usize >= animation.frames.len() {
            match animation.mode {
                AnimationMode::Reverse => unreachable!(),
                AnimationMode::Forward => 0,
                AnimationMode::PingPong => {
                    self.current_animation_direction = AnimationDirection::Reverse;
                    (animation.frames.len() - 2).max(0)
                }
            }
        } else {
            next_frame as usize
        })
    }

    /// If tag is valid, invoke `f` with the current animation frame.
    fn with_current_frame<F, R>(&self, f: F) -> Result<R, InvalidSpriteTag>
    where
        F: Fn(&SpriteFrame) -> R,
    {
        let animation = self
            .animations
            .animations
            .get(&self.current_tag)
            .ok_or(InvalidSpriteTag)?;

        Ok(f(&animation.frames[self.current_frame]))
    }
}

/// A collection of [`SpriteAnimation`]s. This is an immutable object that
/// shares data when cloned to minimize data copies.
#[derive(Debug, Clone)]
pub struct SpriteAnimations {
    animations: Arc<HashMap<Option<String>, SpriteAnimation>>,
}

impl SpriteAnimations {
    /// Creates a new collection from `animations`.
    #[must_use]
    pub fn new(animations: HashMap<Option<String>, SpriteAnimation>) -> Self {
        Self {
            animations: Arc::new(animations),
        }
    }

    /// Returns the animation for `tag`.
    #[must_use]
    pub fn animation_for(&self, tag: &Option<impl ToString>) -> Option<&'_ SpriteAnimation> {
        self.animations.get(&tag.as_ref().map(ToString::to_string))
    }
}

/// An animation of one or more [`SpriteFrame`]s.
#[derive(Debug, Clone)]
pub struct SpriteAnimation {
    /// The frames of the animation.
    pub frames: Arc<Vec<SpriteFrame>>,
    /// The mode of the animation.
    pub mode: AnimationMode,
}

impl SpriteAnimation {
    /// Creates a new animation with `frames` and [`AnimationMode::Forward`].
    #[must_use]
    pub fn new(frames: Vec<SpriteFrame>) -> Self {
        Self {
            frames: Arc::new(frames),
            mode: AnimationMode::Forward,
        }
    }

    /// Builder-style function. Sets `mode` and returns self.
    #[must_use]
    pub const fn with_mode(mut self, mode: AnimationMode) -> Self {
        self.mode = mode;
        self
    }
}

/// A single frame for a [`SpriteAnimation`].
#[derive(Debug, Clone)]
pub struct SpriteFrame {
    /// The source to render.
    pub source: SpriteSource,
    /// The length the frame should be displayed. `None` will act as an infinite
    /// duration.
    pub duration: Option<Duration>,
}

impl SpriteFrame {
    /// Creates a new frame with `source` and no duration.
    #[must_use]
    pub fn new(source: impl Into<SpriteSource>) -> Self {
        Self {
            source: source.into(),
            duration: None,
        }
    }

    /// Builder-style function. Sets `duration` and returns self.
    #[must_use]
    pub const fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }
}

/// A collection of sprites from a single [`ShareableTexture`].
#[derive(Debug, Clone)]
pub struct SpriteSheet<T>
where
    T: Debug,
{
    /// The source texture.
    pub texture: ShareableTexture,
    data: Arc<SpriteSheetData<T>>,
}

#[derive(Debug)]
struct SpriteSheetData<T>
where
    T: Debug,
{
    tile_size: Size<UPx>,
    sprites: HashMap<T, Rect<UPx>>,
}

impl<T> SpriteSheet<T>
where
    T: Debug + Eq + Hash,
{
    /// Creates a new sprite sheet, diving `texture` into a grid of `tile_size`
    /// with `gutter_size` spacing between each row and column. The order of
    /// `tiles` will be read left-to-right, top-to-bottom.
    #[must_use]
    pub fn new(
        texture: impl Into<ShareableTexture>,
        tile_size: Size<UPx>,
        gutter_size: Size<UPx>,
        tiles: Vec<T>,
    ) -> Self {
        let texture = texture.into();
        let dimensions = texture.size() / tile_size;
        Self {
            texture,
            data: Arc::new(SpriteSheetData::from_tiles(
                tiles,
                tile_size,
                gutter_size,
                dimensions,
            )),
        }
    }

    /// Returns the size of the tiles within this sheet.
    #[must_use]
    pub fn tile_size(&self) -> Size<UPx> {
        self.data.tile_size
    }

    /// Returns the sprites identified by each element in `iterator`.
    ///
    /// # Panics
    ///
    /// Panics if a tile isn't found.
    #[must_use]
    pub fn sprites<I: IntoIterator<Item = T>>(&self, iterator: I) -> Vec<SpriteSource> {
        iterator
            .into_iter()
            .map(|tile| {
                let location = self.data.sprites.get(&tile).unwrap();
                SpriteSource::Region(TextureRegion {
                    region: *location,
                    texture: self.texture.clone(),
                })
            })
            .collect()
    }

    /// Returns the sprites identified by each element in `iterator` into a
    /// [`SpriteMap`].
    ///
    /// # Panics
    ///
    /// This function panics if any `T` cannot be found in `self`.
    #[must_use]
    pub fn sprite_map<I: IntoIterator<Item = T>>(&self, iterator: I) -> SpriteMap<T> {
        let map = iterator
            .into_iter()
            .map(|tile| {
                let location = self.data.sprites.get(&tile).expect("missing sprite");
                (
                    tile,
                    SpriteSource::Region(TextureRegion {
                        region: *location,
                        texture: self.texture.clone(),
                    }),
                )
            })
            .collect::<HashMap<_, _>>();
        SpriteMap::new(map)
    }
}

impl<T: Debug + Eq + Hash> SpriteSheetData<T> {
    fn from_tiles(
        tiles: Vec<T>,
        tile_size: Size<UPx>,
        gutters: Size<UPx>,
        dimensions: Size<UPx>,
    ) -> Self {
        let mut sprites = HashMap::new();

        let full_size = tile_size + gutters;
        for (index, tile) in tiles.into_iter().enumerate() {
            let index = UPx::new(index.cast::<u32>());
            let y = index / dimensions.width;
            let x = index - y * dimensions.width;
            sprites.insert(
                tile,
                Rect::new(
                    Point::new(x * full_size.width, y * full_size.height),
                    tile_size,
                ),
            );
        }

        Self { tile_size, sprites }
    }
}

impl<T> SpriteSheet<T>
where
    T: Clone + Debug + Eq + Hash,
{
    /// Returns a collection of all tiles in the sheet  as
    #[must_use]
    pub fn to_sprite_map(&self) -> SpriteMap<T> {
        SpriteMap::new(
            self.data
                .sprites
                .clone()
                .iter()
                .map(|(tile, location)| {
                    (
                        tile.clone(),
                        SpriteSource::Region(TextureRegion {
                            region: *location,
                            texture: self.texture.clone(),
                        }),
                    )
                })
                .collect(),
        )
    }
}

impl<T> SpriteCollection<T> for SpriteSheet<T>
where
    T: Debug + Send + Sync + Eq + Hash,
{
    fn sprite(&self, tile: &T) -> Option<SpriteSource> {
        let location = self.data.sprites.get(tile);
        location.map(|location| {
            SpriteSource::Region(TextureRegion {
                region: *location,
                texture: self.texture.clone(),
            })
        })
    }
}

/// A collection of [`SpriteSource`]s.
#[derive(Debug, Clone)]
pub struct SpriteMap<T> {
    sprites: HashMap<T, SpriteSource>,
}

impl<T> Default for SpriteMap<T> {
    fn default() -> Self {
        Self {
            sprites: HashMap::default(),
        }
    }
}

impl<T> SpriteMap<T>
where
    T: Debug + Eq + Hash,
{
    /// Creates a new collection with `sprites`.
    #[must_use]
    pub fn new(sprites: HashMap<T, SpriteSource>) -> Self {
        Self { sprites }
    }

    /// Creates a collection from `sheet` using `converter` to convert from `O`
    /// to `T`.
    #[must_use]
    pub fn from_foreign_sheet<O: Clone + Debug + Eq + Hash, F: Fn(O) -> T>(
        sheet: &SpriteSheet<O>,
        converter: F,
    ) -> Self {
        let mut map = Self::default();
        map.add_foreign_sheet(sheet, converter);
        map
    }

    /// Adds a collection from `sheet` using `converter` to convert from `O` to
    /// `T`.
    pub fn add_foreign_sheet<O: Clone + Debug + Eq + Hash, F: Fn(O) -> T>(
        &mut self,
        sheet: &SpriteSheet<O>,
        converter: F,
    ) {
        for (tile, sprite) in sheet.to_sprite_map() {
            self.sprites.insert(converter(tile), sprite);
        }
    }
}

impl<T> SpriteMap<T>
where
    T: Clone + Debug + Eq + Hash,
{
    /// Adds all sprites from `sheet`.
    pub fn add_sheet(&mut self, sheet: &SpriteSheet<T>) {
        self.add_foreign_sheet(sheet, |a| a);
    }
}

impl<T> Deref for SpriteMap<T> {
    type Target = HashMap<T, SpriteSource>;

    fn deref(&self) -> &HashMap<T, SpriteSource> {
        &self.sprites
    }
}

impl<T> IntoIterator for SpriteMap<T> {
    type IntoIter = hash_map::IntoIter<T, SpriteSource>;
    type Item = (T, SpriteSource);

    fn into_iter(self) -> Self::IntoIter {
        self.sprites.into_iter()
    }
}

/// A region of a texture that is used as frame in a sprite animation.
#[derive(Debug, Clone)]
pub enum SpriteSource {
    /// The sprite's source is a [`TextureRegion`].
    Region(TextureRegion),
    /// The sprite's source is a [`CollectedTexture`].
    Collected(CollectedTexture),
}

impl SpriteSource {
    /// Returns a [`PreparedGraphic`] that renders this texture at `dest`.
    pub fn prepare<Unit>(&self, dest: Rect<Unit>, graphics: &Graphics<'_>) -> PreparedGraphic<Unit>
    where
        Unit: figures::Unit + Div<i32, Output = Unit>,
        Vertex<Unit>: bytemuck::Pod,
    {
        match self {
            SpriteSource::Region(texture) => texture.prepare(dest, graphics),
            SpriteSource::Collected(texture) => texture.prepare(dest, graphics),
        }
    }
}
impl CanRenderTo for SpriteSource {
    fn can_render_to(&self, kludgine: &Kludgine) -> bool {
        match self {
            SpriteSource::Region(texture) => texture.can_render_to(kludgine),
            SpriteSource::Collected(texture) => texture.can_render_to(kludgine),
        }
    }
}

impl TextureSource for SpriteSource {}

impl sealed::TextureSource for SpriteSource {
    fn id(&self) -> crate::sealed::TextureId {
        match self {
            SpriteSource::Region(texture) => texture.id(),
            SpriteSource::Collected(texture) => texture.id(),
        }
    }

    fn is_mask(&self) -> bool {
        match self {
            SpriteSource::Region(texture) => texture.is_mask(),
            SpriteSource::Collected(texture) => texture.is_mask(),
        }
    }

    fn bind_group(&self, graphics: &impl crate::sealed::KludgineGraphics) -> Arc<wgpu::BindGroup> {
        match self {
            SpriteSource::Region(texture) => texture.bind_group(graphics),
            SpriteSource::Collected(texture) => texture.bind_group(graphics),
        }
    }

    fn default_rect(&self) -> Rect<UPx> {
        match self {
            SpriteSource::Region(texture) => texture.default_rect(),
            SpriteSource::Collected(texture) => texture.default_rect(),
        }
    }
}

impl From<TextureRegion> for SpriteSource {
    fn from(texture: TextureRegion) -> Self {
        Self::Region(texture)
    }
}

impl From<CollectedTexture> for SpriteSource {
    fn from(texture: CollectedTexture) -> Self {
        Self::Collected(texture)
    }
}

/// A collection of sprites.
pub trait SpriteCollection<T>
where
    T: Send + Sync,
{
    /// Returns the sprite referred to by `tile`.
    #[must_use]
    fn sprite(&self, tile: &T) -> Option<SpriteSource>;

    /// Returns all of the requested `tiles`.
    ///
    /// # Panics
    ///
    /// Panics if a tile is not found.
    #[must_use]
    fn sprites(&self, tiles: &[T]) -> Vec<SpriteSource> {
        tiles
            .iter()
            .map(|t| self.sprite(t).unwrap())
            .collect::<Vec<_>>()
    }
}

impl<T> SpriteCollection<T> for SpriteMap<T>
where
    T: Send + Sync + Eq + Hash,
{
    fn sprite(&self, tile: &T) -> Option<SpriteSource> {
        self.sprites.get(tile).cloned()
    }
}
