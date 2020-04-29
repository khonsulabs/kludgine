use super::{
    math::{Point, Rect, Size},
    source_sprite::SourceSprite,
    texture::{LoadedTexture, Texture},
    KludgineError, KludgineHandle, KludgineResult,
};
use futures::lock::Mutex;
use std::{collections::HashMap, sync::Arc, time::Duration};

#[macro_export]
macro_rules! include_aseprite_sprite {
    ($json_path:expr, $image_path:expr) => {
        async {
            let image_bytes = std::include_bytes!($image_path);
            match Texture::from_bytes(image_bytes) {
                Ok(texture) => Sprite::load_aseprite_json(include_str!($json_path), texture).await,
                Err(err) => Err(err),
            }
        }
    };
}

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

#[derive(Clone)]
enum AnimationDirection {
    Forward,
    Reverse,
}

#[derive(Clone)]
pub struct Sprite {
    pub(crate) handle: KludgineHandle<SpriteData>,
}

#[derive(Clone)]
pub(crate) struct SpriteData {
    title: Option<String>,
    elapsed_since_frame_change: Duration,
    current_tag: Option<String>,
    current_frame: usize,
    current_animation_direction: AnimationDirection,
    animations: KludgineHandle<HashMap<Option<String>, SpriteAnimation>>,
}

impl Sprite {
    pub(crate) fn new(
        title: Option<String>,
        animations: KludgineHandle<HashMap<Option<String>, SpriteAnimation>>,
    ) -> Self {
        Self {
            handle: Arc::new(Mutex::new(SpriteData {
                title,
                animations,
                current_frame: 0,
                current_tag: None,
                elapsed_since_frame_change: Duration::from_millis(0),
                current_animation_direction: AnimationDirection::Forward,
            })),
        }
    }

    pub async fn new_instance(&self) -> Self {
        let data = self.handle.lock().await;
        Self {
            handle: Arc::new(Mutex::new(data.clone())),
        }
    }

    pub async fn single_frame(texture: Texture) -> Self {
        let size = texture.size().await;
        let source = SourceSprite::new(
            Rect::sized(Point::default(), Size::new(size.width, size.height)),
            texture,
        );
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
        let frames = Arc::new(Mutex::new(frames));

        Self::new(None, frames)
    }

    /// Loads [Aseprite](https://www.aseprite.org/) JSON export format, when using the correct settings
    ///
    /// For the JSON data, use the Hash export option (default), and use either spaces or underscores (_)
    /// inbetween the fields in the name. Ensure `{frame}` is the last field in the name before the extension.
    /// E.g., `{tag}_{frame}.{extension}`
    pub async fn load_aseprite_json(raw_json: &str, texture: Texture) -> KludgineResult<Self> {
        let json = json::parse(raw_json)?;

        // Validate the data
        let meta = &json["meta"];
        if !meta.is_object() {
            return Err(KludgineError::SpriteParseError(
                "invalid aseprite json: No `meta` section".to_owned(),
            ));
        }

        let texture_size = texture.size().await;
        if meta["size"]["w"] != texture_size.width || meta["size"]["h"] != texture_size.height {
            return Err(KludgineError::SpriteParseError(
                "invalid aseprite json: Size did not match input texture".to_owned(),
            ));
        }

        let title = match meta["image"].as_str() {
            Some(image) => Some(image.to_owned()),
            None => None,
        };

        let mut frames = HashMap::new();
        for (name, frame) in json["frames"].entries() {
            // Remove the extension, if present
            let name = name.split(".").next().unwrap();
            // Split by _ or ' 'as per the documentation of this method.
            let name_parts = name.split(|c| c == '_' || c == ' ').collect::<Vec<_>>();
            let frame_number = name_parts[name_parts.len() - 1]
                .parse::<usize>()
                .map_err(|_| {
                    KludgineError::SpriteParseError(
                        "invalid aseprite json: frame was not numeric.".to_owned(),
                    )
                })?;

            let duration = match frame["duration"].as_u64() {
                Some(millis) => Duration::from_millis(millis),
                None => {
                    return Err(KludgineError::SpriteParseError(
                        "invalid aseprite json: invalid duration".to_owned(),
                    ))
                }
            };

            let frame = Rect::sized(
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

            let source = SourceSprite::new(frame, texture.clone());

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

            let name = match tag["name"].as_str() {
                Some(s) => Some(s.to_owned()),
                None => None,
            };

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
                let frame = frames.get(&i).ok_or(KludgineError::SpriteParseError(
                    "invalid aseprite json: frameTags frame was out of bounds".to_owned(),
                ))?;
                animation_frames.push(frame.clone());
            }

            animations.insert(name, SpriteAnimation::new(animation_frames, direction));
        }

        Ok(Sprite::new(title, Arc::new(Mutex::new(animations))))
    }

    pub async fn set_current_tag<S: Into<String>>(&self, tag: Option<S>) -> KludgineResult<()> {
        let new_tag = tag.map_or(None, |t| Some(t.into()));
        let mut sprite = self.handle.lock().await;
        if sprite.current_tag != new_tag {
            sprite.current_animation_direction = {
                let animations = sprite.animations.lock().await;
                let animation = animations
                    .get(&new_tag)
                    .ok_or_else(|| KludgineError::InvalidSpriteTag)?;
                animation.mode.default_direction()
            };
            sprite.current_frame = 0;
            sprite.current_tag = new_tag;
        }

        Ok(())
    }

    pub async fn get_frame(&self, elapsed: Option<Duration>) -> KludgineResult<SourceSprite> {
        let mut sprite = self.handle.lock().await;
        if let Some(elapsed) = elapsed {
            sprite.elapsed_since_frame_change += elapsed;

            let current_frame_duration = sprite.with_current_frame(|frame| frame.duration).await?;
            if let Some(frame_duration) = current_frame_duration {
                if sprite.elapsed_since_frame_change > frame_duration {
                    sprite.elapsed_since_frame_change = Duration::from_nanos(
                        (sprite.elapsed_since_frame_change.as_nanos() % frame_duration.as_nanos())
                            as u64,
                    );
                    sprite.advance_frame().await?;
                }
            }
        }

        Ok(sprite
            .with_current_frame(|frame| frame.source.clone())
            .await?)
    }
}

impl SpriteData {
    async fn advance_frame(&mut self) -> KludgineResult<()> {
        self.current_frame = self.next_frame().await?;
        Ok(())
    }
    async fn next_frame(&mut self) -> KludgineResult<usize> {
        let starting_frame = self.current_frame as i32;
        let frames = self.animations.lock().await;
        let animation = frames
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

    async fn with_current_frame<F, R>(&self, f: F) -> KludgineResult<R>
    where
        F: Fn(&SpriteFrame) -> R,
    {
        let frames = self.animations.lock().await;
        let animation = frames
            .get(&self.current_tag)
            .ok_or(KludgineError::InvalidSpriteTag)?;

        Ok(f(&animation.frames[self.current_frame]))
    }
}

pub struct SpriteAnimation {
    frames: Vec<SpriteFrame>,
    mode: AnimationMode,
}

impl SpriteAnimation {
    pub fn new(frames: Vec<SpriteFrame>, mode: AnimationMode) -> Self {
        Self { frames, mode }
    }
}

#[derive(Clone)]
pub struct SpriteFrame {
    source: SourceSprite,
    duration: Option<Duration>,
}

pub struct SpriteFrameBuilder {
    source: SourceSprite,
    tag: Option<String>,
    tag_frame: Option<usize>,
    duration: Option<Duration>,
}

impl SpriteFrameBuilder {
    pub fn new(source: SourceSprite) -> Self {
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

#[derive(Clone)]
pub(crate) struct RenderedSprite {
    pub(crate) handle: KludgineHandle<RenderedSpriteData>,
}

impl RenderedSprite {
    pub fn new(render_at: Rect, source: SourceSprite) -> Self {
        Self {
            handle: Arc::new(Mutex::new(RenderedSpriteData { render_at, source })),
        }
    }
}

pub(crate) struct RenderedSpriteData {
    pub render_at: Rect,
    pub source: SourceSprite,
}

pub(crate) struct SpriteBatch {
    pub loaded_texture: LoadedTexture,
    pub sprites: Vec<RenderedSprite>,
}

impl SpriteBatch {
    pub fn new(loaded_texture: LoadedTexture) -> Self {
        SpriteBatch {
            loaded_texture,
            sprites: Vec::new(),
        }
    }
}
