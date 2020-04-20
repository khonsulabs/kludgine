use super::{
    math::Rect,
    source_sprite::SourceSprite,
    texture::{LoadedTexture, Texture},
    KludgineError, KludgineHandle, KludgineResult,
};
use std::{collections::HashMap, time::Duration};

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
    frames: KludgineHandle<HashMap<Option<String>, Vec<SpriteFrame>>>,
}

impl Sprite {
    pub(crate) fn new(
        title: Option<String>,
        frames: KludgineHandle<HashMap<Option<String>, Vec<SpriteFrame>>>,
    ) -> Self {
        Self {
            handle: KludgineHandle::new(SpriteData {
                title,
                frames,
                current_frame: 0,
                current_tag: None,
                elapsed_since_frame_change: Duration::from_millis(0),
            }),
        }
    }

    pub fn new_instance(&self) -> Self {
        let data = self.handle.read().expect("Error locking sprite to copy");
        Self {
            handle: KludgineHandle::new(data.clone()),
        }
    }

    pub fn single_frame(texture: Texture) -> Self {
        let size = texture.size();
        let source = SourceSprite::new(Rect::sized(0, 0, size.width, size.height), texture);
        let mut frames = HashMap::new();
        frames.insert(
            None,
            vec![SpriteFrame {
                source,
                tag: None,
                tag_frame: 0,
                duration: None,
            }],
        );
        let frames = KludgineHandle::new(frames);

        Self::new(None, frames)
    }

    /// Loads [Aseprite](https://www.aseprite.org/) JSON export format, when using the correct settings
    ///
    /// For the JSON data, use the item name of {title}_{tag}_{tagframe}.{extension}
    pub fn load_aseprite_json(raw_json: &str, texture: Texture) -> KludgineResult<Self> {
        let json = json::parse(raw_json)?;

        // Validate the data
        let meta = &json["meta"];
        if !meta.is_object() {
            return Err(KludgineError::SpriteParseError(
                "invalid aseprite json: No `meta` section".to_owned(),
            ));
        }
        // TODO Validate that the texture size matches the JSON size

        let mut title = None;
        let mut frames = HashMap::new();

        for (name, frame) in json["frames"].entries() {
            // Remove the extension, if present
            let name = name.split(".").next().unwrap();
            // Split by _ as per the documentation of this method.
            let name_parts = name.split("_").collect::<Vec<_>>();
            if name_parts.len() != 3 {
                return Err(KludgineError::SpriteParseError(
                    "invalid aseprite json: Frame name does not match the {title}_{tag}_{tagframe}.{extension} format".to_owned(),
                ));
            }

            title = Some(name_parts[0].to_owned());
            let tag = name_parts[1].to_owned();
            let tag_frame = match name_parts[2].parse::<usize>() {
                Ok(frame) => frame,
                Err(_) => {
                    return Err(KludgineError::SpriteParseError(
                        "invalid aseprite json: tagframe was not numeric.".to_owned(),
                    ))
                }
            };

            let duration = match frame["duration"].as_u64() {
                Some(millis) => Duration::from_millis(millis),
                None => {
                    return Err(KludgineError::SpriteParseError(
                        "invalid aseprite json: invalid duration".to_owned(),
                    ))
                }
            };

            let frame = Rect::sized(
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
            );

            let source = SourceSprite::new(frame, texture.clone());

            let tag_frames = frames
                .entry(Some(tag.clone()))
                .or_insert_with(|| Vec::new());

            // TODO Insert sorted
            tag_frames.push(SpriteFrame {
                tag: Some(tag),
                tag_frame: tag_frame,
                duration: Some(duration),
                source,
            });
        }

        Ok(Sprite::new(title, KludgineHandle::new(frames)))
    }

    pub fn set_current_tag<S: Into<String>>(&self, tag: Option<S>) -> KludgineResult<()> {
        let new_tag = tag.map_or(None, |t| Some(t.into()));
        let mut sprite = self.handle.write().expect("Error locking sprite");
        if sprite.current_tag != new_tag {
            sprite.current_frame = 0;
            sprite.current_tag = new_tag;
        }

        Ok(())
    }

    pub fn get_frame(&self, elapsed: Option<Duration>) -> KludgineResult<SourceSprite> {
        let mut sprite = self.handle.write().expect("Error locking sprite");
        if let Some(elapsed) = elapsed {
            sprite.elapsed_since_frame_change += elapsed;

            let current_frame_duration = sprite.with_current_frame(|frame| frame.duration)?;
            if let Some(frame_duration) = current_frame_duration {
                if sprite.elapsed_since_frame_change > frame_duration {
                    sprite.elapsed_since_frame_change = Duration::from_nanos(
                        (sprite.elapsed_since_frame_change.as_nanos() % frame_duration.as_nanos())
                            as u64,
                    );
                    sprite.advance_frame()?;
                }
            }
        }

        Ok(sprite.with_current_frame(|frame| frame.source.clone())?)
    }
}

impl SpriteData {
    fn advance_frame(&mut self) -> KludgineResult<()> {
        self.current_frame = self.next_frame()?;
        Ok(())
    }
    fn next_frame(&mut self) -> KludgineResult<usize> {
        let starting_frame = self.current_frame;
        let frames = self
            .frames
            .read()
            .expect("Error locking frames for reading");
        let tag_frames = frames
            .get(&self.current_tag)
            .ok_or(KludgineError::InvalidSpriteTag)?;

        for i in (starting_frame + 1)..tag_frames.len() {
            if tag_frames[i].tag == self.current_tag {
                return Ok(i);
            }
        }

        for i in 0..(starting_frame + 1) {
            if tag_frames[i].tag == self.current_tag {
                return Ok(i);
            }
        }

        unreachable!()
    }

    fn with_current_frame<F, R>(&self, f: F) -> KludgineResult<R>
    where
        F: Fn(&SpriteFrame) -> R,
    {
        let frames = self
            .frames
            .read()
            .expect("Error locking frames for reading");
        let tag_frames = frames
            .get(&self.current_tag)
            .ok_or(KludgineError::InvalidSpriteTag)?;

        Ok(f(&tag_frames[self.current_frame]))
    }
}

pub struct SpriteFrame {
    tag: Option<String>,
    tag_frame: usize,
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
            tag: self.tag,
            tag_frame: self.tag_frame.unwrap_or_default(),
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
            handle: KludgineHandle::new(RenderedSpriteData { render_at, source }),
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
