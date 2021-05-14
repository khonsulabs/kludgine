use crate::{
    math::{Angle, Box2D, Point, Raw, Rect, Scale, Size},
    texture::Texture,
    KludgineError, KludgineResult,
};
mod batch;
mod collection;
mod gpu_batch;
mod pipeline;
mod sheet;
#[cfg(target_arch = "wasm32")]
pub(crate) use self::pipeline::Normal;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use self::pipeline::Srgb;
pub(crate) use self::{
    batch::Batch,
    gpu_batch::{BatchBuffers, GpuBatch},
    pipeline::{Pipeline, VertexShaderSource},
};

mod source;
pub use self::{collection::*, sheet::*, source::*};
use std::{collections::HashMap, iter::IntoIterator, sync::Arc, time::Duration};

#[macro_export]
macro_rules! include_aseprite_sprite {
    ($path:expr) => {{
        let image_bytes = std::include_bytes!(concat!($path, ".png"));
        match Texture::from_bytes(image_bytes) {
            Ok(texture) => {
                Sprite::load_aseprite_json(include_str!(concat!($path, ".json")), texture)
            }
            Err(err) => Err(err),
        }
    }};
}

#[derive(Debug, Clone)]
pub enum AnimationMode {
    Forward,
    Reverse,
    PingPong,
}

impl AnimationMode {
    fn default_direction(&self) -> AnimationDirection {
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

#[derive(Debug, Clone)]
pub struct Sprite {
    title: Option<String>,
    elapsed_since_frame_change: Duration,
    current_tag: Option<String>,
    current_frame: usize,
    current_animation_direction: AnimationDirection,
    animations: SpriteAnimations,
}

impl From<SpriteAnimations> for Sprite {
    fn from(animations: SpriteAnimations) -> Self {
        Self::new(None, animations)
    }
}

impl Sprite {
    pub fn new(title: Option<String>, animations: SpriteAnimations) -> Self {
        Self {
            title,
            animations,
            current_frame: 0,
            current_tag: None,
            elapsed_since_frame_change: Duration::from_millis(0),
            current_animation_direction: AnimationDirection::Forward,
        }
    }

    /// For merging multiple Sprites that have no tags within them
    pub fn merged<S: Into<String>, I: IntoIterator<Item = (S, Self)>>(source: I) -> Self {
        let mut combined = HashMap::new();
        let mut title = None;
        for (name, sprite) in source {
            combined.insert(
                Some(name.into()),
                sprite
                    .animations
                    .frames_for(&Option::<&str>::None)
                    .unwrap()
                    .clone(),
            );
            title = title.or_else(|| sprite.title.clone());
        }
        Self::new(title, SpriteAnimations::new(combined))
    }

    pub fn single_frame(texture: Texture) -> Self {
        let size = texture.size();
        let source = SpriteSource::new(Rect::new(Point::default(), size.cast_unit()), texture);
        let mut frames = HashMap::new();
        frames.insert(
            None,
            SpriteAnimation::new(
                vec![SpriteFrame {
                    source,
                    duration: None,
                }],
                AnimationMode::Forward,
            ),
        );
        let frames = SpriteAnimations::new(frames);

        Self::new(None, frames)
    }

    /// Loads [Aseprite](https://www.aseprite.org/) JSON export format, when using the correct settings
    ///
    /// For the JSON data, use the Hash export option (default), and use either spaces or underscores (_)
    /// inbetween the fields in the name. Ensure `{frame}` is the last field in the name before the extension.
    /// E.g., `{tag}_{frame}.{extension}`
    pub fn load_aseprite_json(raw_json: &str, texture: Texture) -> KludgineResult<Self> {
        let json = json::parse(raw_json)?;

        // Validate the data
        let meta = &json["meta"];
        if !meta.is_object() {
            return Err(KludgineError::SpriteParseError(
                "invalid aseprite json: No `meta` section".to_owned(),
            ));
        }

        let texture_size = texture.size();
        if meta["size"]["w"] != texture_size.width || meta["size"]["h"] != texture_size.height {
            return Err(KludgineError::SpriteParseError(
                "invalid aseprite json: Size did not match input texture".to_owned(),
            ));
        }

        let title = meta["image"].as_str().map(str::to_owned);

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
                        Err(KludgineError::SpriteParseError(
                            "invalid aseprite json: frame was not numeric.".to_owned(),
                        ))
                    }
                })?;

            let duration = match frame["duration"].as_u64() {
                Some(millis) => Duration::from_millis(millis),
                None => {
                    return Err(KludgineError::SpriteParseError(
                        "invalid aseprite json: invalid duration".to_owned(),
                    ))
                }
            };

            let frame = Rect::new(
                Point::new(
                    frame["frame"]["x"].as_u32().ok_or_else(|| {
                        KludgineError::SpriteParseError(
                            "invalid aseprite json: frame x was not valid".to_owned(),
                        )
                    })?,
                    frame["frame"]["y"].as_u32().ok_or_else(|| {
                        KludgineError::SpriteParseError(
                            "invalid aseprite json: frame y was not valid".to_owned(),
                        )
                    })?,
                ),
                Size::new(
                    frame["frame"]["w"].as_u32().ok_or_else(|| {
                        KludgineError::SpriteParseError(
                            "invalid aseprite json: frame w was not valid".to_owned(),
                        )
                    })?,
                    frame["frame"]["h"].as_u32().ok_or_else(|| {
                        KludgineError::SpriteParseError(
                            "invalid aseprite json: frame h was not valid".to_owned(),
                        )
                    })?,
                ),
            );

            let source = SpriteSource::new(frame, texture.clone());

            frames.insert(
                frame_number,
                SpriteFrame {
                    duration: Some(duration),
                    source,
                },
            );
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
                return Err(KludgineError::SpriteParseError(
                    "invalid aseprite json: frameTags direction is an unknown value".to_owned(),
                ));
            };

            let name = tag["name"].as_str().map(str::to_owned);

            let start_frame = tag["from"].as_usize().ok_or_else(|| {
                KludgineError::SpriteParseError(
                    "invalid aseprite json: frameTags from was not numeric".to_owned(),
                )
            })?;
            let end_frame = tag["to"].as_usize().ok_or_else(|| {
                KludgineError::SpriteParseError(
                    "invalid aseprite json: frameTags from was not numeric".to_owned(),
                )
            })?;
            let mut animation_frames = Vec::new();
            for i in start_frame..(end_frame + 1) {
                let frame = frames.get(&i).ok_or_else(|| {
                    KludgineError::SpriteParseError(
                        "invalid aseprite json: frameTags frame was out of bounds".to_owned(),
                    )
                })?;
                animation_frames.push(frame.clone());
            }

            animations.insert(name, SpriteAnimation::new(animation_frames, direction));
        }

        let mut frames: Vec<_> = frames.into_iter().collect();
        frames.sort_by(|a, b| a.0.cmp(&b.0));

        animations.insert(
            None,
            SpriteAnimation::new(
                frames.iter().map(|(_, f)| f.clone()).collect(),
                AnimationMode::Forward,
            ),
        );

        Ok(Sprite::new(title, SpriteAnimations::new(animations)))
    }

    pub fn set_current_tag<S: Into<String>>(&mut self, tag: Option<S>) -> KludgineResult<()> {
        let new_tag = tag.map(|t| t.into());
        if self.current_tag != new_tag {
            self.current_animation_direction = {
                let animation = self
                    .animations
                    .animations
                    .get(&new_tag)
                    .ok_or(KludgineError::InvalidSpriteTag)?;
                animation.mode.default_direction()
            };
            self.current_frame = 0;
            self.current_tag = new_tag;
        }

        Ok(())
    }

    pub fn current_tag(&self) -> Option<&'_ str> {
        self.current_tag.as_deref()
    }

    pub fn get_frame(&mut self, elapsed: Option<Duration>) -> KludgineResult<SpriteSource> {
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

    pub fn remaining_frame_duration(&self) -> KludgineResult<Option<Duration>> {
        let duration = self
            .with_current_frame(|frame| frame.duration)?
            .map(|frame_duration| {
                frame_duration
                    .checked_sub(self.elapsed_since_frame_change)
                    .unwrap_or_default()
            });

        Ok(duration)
    }

    pub fn animations(&self) -> &'_ SpriteAnimations {
        &self.animations
    }

    pub fn bounds(&self) -> Option<Rect<u32>> {
        if let Some(animation) = self.animations.animations.values().next() {
            if let Some(frame) = animation.frames.first() {
                return Some(frame.source.location.bounds());
            }
        }
        None
    }

    pub fn size(&self) -> Option<Size<u32>> {
        if let Some(animation) = self.animations.animations.values().next() {
            if let Some(frame) = animation.frames.first() {
                return Some(frame.source.location.size());
            }
        }
        None
    }

    fn advance_frame(&mut self) -> KludgineResult<()> {
        self.current_frame = self.next_frame()?;
        Ok(())
    }

    fn next_frame(&mut self) -> KludgineResult<usize> {
        let starting_frame = self.current_frame as i32;
        let animation = self
            .animations
            .animations
            .get(&self.current_tag)
            .ok_or(KludgineError::InvalidSpriteTag)?;

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

    fn with_current_frame<F, R>(&self, f: F) -> KludgineResult<R>
    where
        F: Fn(&SpriteFrame) -> R,
    {
        let animation = self
            .animations
            .animations
            .get(&self.current_tag)
            .ok_or(KludgineError::InvalidSpriteTag)?;

        Ok(f(&animation.frames[self.current_frame]))
    }
}

#[derive(Clone, Debug)]
pub struct SpriteAnimations {
    animations: Arc<HashMap<Option<String>, SpriteAnimation>>,
}

impl SpriteAnimations {
    pub fn new(animations: HashMap<Option<String>, SpriteAnimation>) -> Self {
        Self {
            animations: Arc::new(animations),
        }
    }

    pub fn frames_for(&self, tag: &Option<impl ToString>) -> Option<&'_ SpriteAnimation> {
        self.animations.get(&tag.as_ref().map(|s| s.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct SpriteAnimation {
    pub frames: Vec<SpriteFrame>,
    pub mode: AnimationMode,
}

impl SpriteAnimation {
    pub fn new(frames: Vec<SpriteFrame>, mode: AnimationMode) -> Self {
        Self { frames, mode }
    }
}

#[derive(Debug, Clone)]
pub struct SpriteFrame {
    pub source: SpriteSource,
    pub duration: Option<Duration>,
}

pub struct SpriteFrameBuilder {
    source: SpriteSource,
    tag: Option<String>,
    tag_frame: Option<usize>,
    duration: Option<Duration>,
}

impl SpriteFrameBuilder {
    pub fn new(source: SpriteSource) -> Self {
        Self {
            source,
            tag: None,
            tag_frame: None,
            duration: None,
        }
    }

    pub fn with_tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn with_tag_frame(mut self, frame: usize) -> Self {
        self.tag_frame = Some(frame);
        self
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn build(self) -> SpriteFrame {
        SpriteFrame {
            source: self.source,
            duration: self.duration,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RenderedSprite {
    pub(crate) data: Arc<RenderedSpriteData>,
}

impl RenderedSprite {
    pub fn new(
        render_at: Box2D<f32, Raw>,
        rotation: SpriteRotation<Raw>,
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
    pub render_at: Box2D<f32, Raw>,
    pub rotation: SpriteRotation<Raw>,
    pub alpha: f32,
    pub source: SpriteSource,
}

#[derive(Copy, Clone, Debug)]
pub struct SpriteRotation<Unit> {
    pub angle: Option<Angle>,
    /// The location to rotate the sprite around. If not specified, the center of the sprite is used.
    pub screen_location: Option<Point<f32, Unit>>,
}

impl<Unit> Default for SpriteRotation<Unit> {
    fn default() -> Self {
        Self {
            angle: None,
            screen_location: None,
        }
    }
}

impl<Unit> SpriteRotation<Unit> {
    pub fn around_center(angle: Angle) -> Self {
        Self {
            angle: Some(angle),
            screen_location: None,
        }
    }

    pub fn around(angle: Angle, screen_location: Point<f32, Unit>) -> Self {
        Self {
            angle: Some(angle),
            screen_location: Some(screen_location),
        }
    }
}

impl<A, B> std::ops::Mul<Scale<f32, A, B>> for SpriteRotation<A> {
    type Output = SpriteRotation<B>;

    fn mul(self, rhs: Scale<f32, A, B>) -> Self::Output {
        SpriteRotation {
            angle: self.angle,
            screen_location: self.screen_location.map(|l| l * rhs),
        }
    }
}
