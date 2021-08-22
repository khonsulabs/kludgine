use figures::{Displayable, Pixels, Points, Scaled};

use crate::{
    math::{Angle, ExtentsRect, Point, Rect, Size},
    texture::Texture,
    Error,
};
mod batch;
mod collection;
mod gpu_batch;
mod pipeline;
mod sheet;
pub(crate) use self::{
    batch::Batch,
    gpu_batch::{BatchBuffers, GpuBatch},
    pipeline::Pipeline,
};

mod source;
use std::{collections::HashMap, iter::IntoIterator, sync::Arc, time::Duration};

pub use self::{collection::*, pipeline::VertexShaderSource, sheet::*, source::*};

/// Includes an [Aseprite](https://www.aseprite.org/) sprite sheet and Json
/// export. For more information, see [`Sprite::load_aseprite_json`]. This macro
/// will append ".png" and ".json" to the path provided and include both files
/// in your binary.
#[macro_export]
macro_rules! include_aseprite_sprite {
    ($path:expr) => {{
        $crate::include_texture!(concat!($path, ".png")).and_then(|texture| {
            $crate::sprite::Sprite::load_aseprite_json(
                include_str!(concat!($path, ".json")),
                &texture,
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
            combined.insert(
                Some(name.into()),
                sprite
                    .animations
                    .animation_for(&Option::<&str>::None)
                    .unwrap()
                    .clone(),
            );
        }
        Self::new(SpriteAnimations::new(combined))
    }

    /// Creates an instance from a texture. This creates a `SpriteAnimation`
    /// with no tag and a single frame.
    #[must_use]
    pub fn single_frame(texture: Texture) -> Self {
        let source = SpriteSource::entire_texture(texture);
        let mut frames = HashMap::new();
        frames.insert(
            None,
            SpriteAnimation::new(vec![SpriteFrame {
                source,
                duration: None,
            }])
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
    #[allow(clippy::too_many_lines)]
    // TODO refactor. Now that I know more about serde, this probably can be parsed
    // with a complex serde type.
    pub fn load_aseprite_json(raw_json: &str, texture: &Texture) -> crate::Result<Self> {
        let json = json::parse(raw_json)?;

        // Validate the data
        let meta = &json["meta"];
        if !meta.is_object() {
            return Err(Error::SpriteParse(
                "invalid aseprite json: No `meta` section".to_owned(),
            ));
        }

        let texture_size = texture.size();
        if meta["size"]["w"] != texture_size.width || meta["size"]["h"] != texture_size.height {
            return Err(Error::SpriteParse(
                "invalid aseprite json: Size did not match input texture".to_owned(),
            ));
        }

        let mut frames = HashMap::new();
        for (name, frame) in json["frames"].entries() {
            // Remove the extension, if present
            let name = name.split('.').next().unwrap();
            // Split by _ or ' 'as per the documentation of this method.
            let name_parts = name.split(|c| c == '_' || c == ' ').collect::<Vec<_>>();
            let frame_number = name_parts[name_parts.len() - 1]
                .parse::<usize>()
                .or_else(|_| {
                    if json["frames"].len() == 1 {
                        Ok(0)
                    } else {
                        Err(Error::SpriteParse(
                            "invalid aseprite json: frame was not numeric.".to_owned(),
                        ))
                    }
                })?;

            let duration = match frame["duration"].as_u64() {
                Some(millis) => Duration::from_millis(millis),
                None =>
                    return Err(Error::SpriteParse(
                        "invalid aseprite json: invalid duration".to_owned(),
                    )),
            };

            let frame = Rect::new(
                Point::new(
                    frame["frame"]["x"].as_u32().ok_or_else(|| {
                        Error::SpriteParse(
                            "invalid aseprite json: frame x was not valid".to_owned(),
                        )
                    })?,
                    frame["frame"]["y"].as_u32().ok_or_else(|| {
                        Error::SpriteParse(
                            "invalid aseprite json: frame y was not valid".to_owned(),
                        )
                    })?,
                ),
                Size::new(
                    frame["frame"]["w"].as_u32().ok_or_else(|| {
                        Error::SpriteParse(
                            "invalid aseprite json: frame w was not valid".to_owned(),
                        )
                    })?,
                    frame["frame"]["h"].as_u32().ok_or_else(|| {
                        Error::SpriteParse(
                            "invalid aseprite json: frame h was not valid".to_owned(),
                        )
                    })?,
                ),
            );

            let source = SpriteSource::new(frame, texture.clone());

            frames.insert(frame_number, SpriteFrame {
                duration: Some(duration),
                source,
            });
        }

        let mut animations = HashMap::new();
        for tag in meta["frameTags"].members() {
            let direction = if tag["direction"] == "forward" {
                AnimationMode::Forward
            } else if tag["direction"] == "reverse" {
                AnimationMode::Reverse
            } else if tag["direction"] == "pingpong" {
                AnimationMode::PingPong
            } else {
                return Err(Error::SpriteParse(
                    "invalid aseprite json: frameTags direction is an unknown value".to_owned(),
                ));
            };

            let name = tag["name"].as_str().map(str::to_owned);

            let start_frame = tag["from"].as_usize().ok_or_else(|| {
                Error::SpriteParse(
                    "invalid aseprite json: frameTags from was not numeric".to_owned(),
                )
            })?;
            let end_frame = tag["to"].as_usize().ok_or_else(|| {
                Error::SpriteParse(
                    "invalid aseprite json: frameTags from was not numeric".to_owned(),
                )
            })?;
            let mut animation_frames = Vec::new();
            for i in start_frame..=end_frame {
                let frame = frames.get(&i).ok_or_else(|| {
                    Error::SpriteParse(
                        "invalid aseprite json: frameTags frame was out of bounds".to_owned(),
                    )
                })?;
                animation_frames.push(frame.clone());
            }

            animations.insert(
                name,
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
    pub fn set_current_tag<S: Into<String>>(&mut self, tag: Option<S>) -> crate::Result<()> {
        let new_tag = tag.map(Into::into);
        if self.current_tag != new_tag {
            self.current_animation_direction = {
                let animation = self
                    .animations
                    .animations
                    .get(&new_tag)
                    .ok_or(Error::InvalidSpriteTag)?;
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
    pub fn get_frame(&mut self, elapsed: Option<Duration>) -> crate::Result<SpriteSource> {
        if let Some(elapsed) = elapsed {
            self.elapsed_since_frame_change += elapsed;

            let current_frame_duration = self.with_current_frame(|frame| frame.duration)?;
            if let Some(frame_duration) = current_frame_duration {
                if self.elapsed_since_frame_change > frame_duration {
                    self.elapsed_since_frame_change = Duration::from_nanos(
                        (self.elapsed_since_frame_change.as_nanos() % frame_duration.as_nanos())
                            as u64,
                    );
                    self.advance_frame()?;
                }
            }
        }

        self.with_current_frame(|frame| frame.source.clone())
    }

    /// Returns the amount of time remaining until the next frame is due to be
    /// shown for this sprite. Can be used to calculate redraws more efficiently
    /// if you're not rendering at a constant framerate.
    pub fn remaining_frame_duration(&self) -> crate::Result<Option<Duration>> {
        let duration = self
            .with_current_frame(|frame| frame.duration)?
            .map(|frame_duration| {
                frame_duration
                    .checked_sub(self.elapsed_since_frame_change)
                    .unwrap_or_default()
            });

        Ok(duration)
    }

    fn advance_frame(&mut self) -> crate::Result<()> {
        self.current_frame = self.next_frame()?;
        Ok(())
    }

    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    fn next_frame(&mut self) -> crate::Result<usize> {
        let starting_frame = self.current_frame as i32;
        let animation = self
            .animations
            .animations
            .get(&self.current_tag)
            .ok_or(Error::InvalidSpriteTag)?;

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

    fn with_current_frame<F, R>(&self, f: F) -> crate::Result<R>
    where
        F: Fn(&SpriteFrame) -> R,
    {
        let animation = self
            .animations
            .animations
            .get(&self.current_tag)
            .ok_or(Error::InvalidSpriteTag)?;

        Ok(f(&animation.frames[self.current_frame]))
    }
}

/// A collection of [`SpriteAnimation`]s. This is an immutable object that
/// shares data when cloned to minimize data copies.
#[derive(Clone, Debug)]
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
        self.animations.get(&tag.as_ref().map(|s| s.to_string()))
    }
}

/// An animation of one or more [`SpriteFrame`]s.
#[derive(Debug, Clone)]
pub struct SpriteAnimation {
    /// The frames of the animation.
    pub frames: Vec<SpriteFrame>,
    /// The mode of the animation.
    pub mode: AnimationMode,
}

impl SpriteAnimation {
    /// Creates a new animation with `frames` and [`AnimationMode::Forward`].
    #[must_use]
    pub fn new(frames: Vec<SpriteFrame>) -> Self {
        Self {
            frames,
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
    pub const fn new(source: SpriteSource) -> Self {
        Self {
            source,
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

/// A rendered sprite.
#[derive(Clone, Debug)]
pub struct RenderedSprite {
    pub(crate) data: Arc<RenderedSpriteData>,
}

impl RenderedSprite {
    #[must_use]
    pub(crate) fn new(
        render_at: ExtentsRect<f32, Pixels>,
        rotation: SpriteRotation<Pixels>,
        alpha: f32,
        source: SpriteSource,
    ) -> Self {
        Self {
            data: Arc::new(RenderedSpriteData {
                render_at,
                rotation,
                alpha,
                source,
            }),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RenderedSpriteData {
    pub render_at: ExtentsRect<f32, Pixels>,
    pub rotation: SpriteRotation<Pixels>,
    pub alpha: f32,
    pub source: SpriteSource,
}

/// A rotation of a sprite.
#[derive(Copy, Clone, Debug)]
#[must_use]
pub struct SpriteRotation<Unit = Scaled> {
    /// The angle to rotate around `screen_location`.
    pub angle: Option<Angle>,
    /// The location to rotate the sprite around. If not specified, the center
    /// of the sprite is used.
    pub location: Option<Point<f32, Unit>>,
}

impl SpriteRotation<Pixels> {
    /// Returns a value that performs no rotation.
    pub const fn none() -> Self {
        Self {
            angle: None,
            location: None,
        }
    }
}

impl<Unit> Default for SpriteRotation<Unit> {
    fn default() -> Self {
        Self {
            angle: None,
            location: None,
        }
    }
}

impl<Unit> SpriteRotation<Unit> {
    /// Returns a rotation around the center of the shape.
    pub const fn around_center(angle: Angle) -> Self {
        Self {
            angle: Some(angle),
            location: None,
        }
    }

    /// Returns a rotation around `location`.
    pub const fn around(angle: Angle, location: Point<f32, Unit>) -> Self {
        Self {
            angle: Some(angle),
            location: Some(location),
        }
    }
}

impl Displayable<f32> for SpriteRotation<Pixels> {
    type Pixels = Self;
    type Points = SpriteRotation<Points>;
    type Scaled = SpriteRotation<Scaled>;

    fn to_pixels(&self, _scale: &figures::DisplayScale<f32>) -> Self::Pixels {
        *self
    }

    fn to_points(&self, scale: &figures::DisplayScale<f32>) -> Self::Points {
        SpriteRotation {
            angle: self.angle,
            location: self.location.map(|l| l.to_points(scale)),
        }
    }

    fn to_scaled(&self, scale: &figures::DisplayScale<f32>) -> Self::Scaled {
        SpriteRotation {
            angle: self.angle,
            location: self.location.map(|l| l.to_scaled(scale)),
        }
    }
}

impl Displayable<f32> for SpriteRotation<Points> {
    type Pixels = SpriteRotation<Pixels>;
    type Points = Self;
    type Scaled = SpriteRotation<Scaled>;

    fn to_pixels(&self, scale: &figures::DisplayScale<f32>) -> Self::Pixels {
        SpriteRotation {
            angle: self.angle,
            location: self.location.map(|l| l.to_pixels(scale)),
        }
    }

    fn to_points(&self, _scale: &figures::DisplayScale<f32>) -> Self::Points {
        *self
    }

    fn to_scaled(&self, scale: &figures::DisplayScale<f32>) -> Self::Scaled {
        SpriteRotation {
            angle: self.angle,
            location: self.location.map(|l| l.to_scaled(scale)),
        }
    }
}

impl Displayable<f32> for SpriteRotation<Scaled> {
    type Pixels = SpriteRotation<Pixels>;
    type Points = SpriteRotation<Points>;
    type Scaled = Self;

    fn to_pixels(&self, scale: &figures::DisplayScale<f32>) -> Self::Pixels {
        SpriteRotation {
            angle: self.angle,
            location: self.location.map(|l| l.to_pixels(scale)),
        }
    }

    fn to_points(&self, scale: &figures::DisplayScale<f32>) -> Self::Points {
        SpriteRotation {
            angle: self.angle,
            location: self.location.map(|l| l.to_points(scale)),
        }
    }

    fn to_scaled(&self, _scale: &figures::DisplayScale<f32>) -> Self::Scaled {
        *self
    }
}

/// The Srgb colorspace. Used as a `VertexShaderSource` in
/// [`FrameRenderer`](crate::frame_renderer::FrameRenderer).
pub struct Srgb;
/// The uncorrected Rgb colorspace. Used as a
/// [`VertexShaderSource`](crate::sprite::VertexShaderSource) in
/// [`FrameRenderer`](crate::frame_renderer::FrameRenderer).
pub struct Normal;
