use std::collections::{HashMap, HashSet};

use easygpu::transform::ScreenSpace;
use euclid::Rect;

use crate::{
    math::{Raw, Size},
    scene::{Element, SceneEvent},
    shape, sprite,
    text::{font::LoadedFont, prepared::PreparedSpan},
    texture::Texture,
};
#[derive(Default, Debug)]
pub(crate) struct Frame {
    pub size: Size<f32, ScreenSpace>,
    pub commands: Vec<FrameCommand>,
    pub(crate) textures: HashMap<u64, Texture>,
    pub(crate) fonts: HashMap<u64, LoadedFont>,
    pub(crate) pending_font_updates: Vec<FontUpdate>,
    receiver: FrameReceiver,
}

#[derive(Default, Debug)]
struct FrameReceiver {
    size: Size<f32, ScreenSpace>,
    elements: Vec<Element>,
}

impl FrameReceiver {
    pub fn get_latest_frame(&mut self, receiver: &flume::Receiver<SceneEvent>) -> Vec<Element> {
        // Receive a frame, blocking until we get an EndFrame
        while let Ok(evt) = receiver.recv() {
            if self.process_scene_event(evt) {
                // New frame
                break;
            }
        }
        let mut latest_frame = std::mem::replace(&mut self.elements, Vec::new());
        // Receive any pending events in a non-blocking fashion. We only want to render
        // the frame we have the most recent information for
        while let Ok(evt) = receiver.try_recv() {
            if self.process_scene_event(evt) {
                // New frame
                latest_frame = std::mem::replace(&mut self.elements, Vec::new());
            }
        }
        latest_frame
    }

    fn process_scene_event(&mut self, event: SceneEvent) -> bool {
        match event {
            SceneEvent::BeginFrame { size } => {
                self.size = size.cast_unit();
                false
            }
            SceneEvent::Render(element) => {
                self.elements.push(element);
                false
            }

            SceneEvent::EndFrame => true,
        }
    }
}

#[derive(Debug)]
pub(crate) struct FontUpdate {
    pub font_id: u64,
    pub rect: rusttype::Rect<u32>,
    pub data: Vec<u8>,
}

enum FrameBatch {
    Sprite(sprite::Batch),
    Shape(shape::Batch),
}

impl FrameBatch {
    fn is_shape(&self) -> bool {
        matches!(self, Self::Shape(_))
    }

    fn is_sprite(&self) -> bool {
        !self.is_shape()
    }

    fn sprite_batch(&self) -> Option<&'_ sprite::Batch> {
        if let FrameBatch::Sprite(batch) = self {
            Some(batch)
        } else {
            None
        }
    }

    // fn shape_batch(&self) -> Option<&'_ shape::Batch> {
    //     if let FrameBatch::Shape(batch) = self {
    //         Some(batch)
    //     } else {
    //         None
    //     }
    // }

    fn sprite_batch_mut(&mut self) -> Option<&'_ mut sprite::Batch> {
        if let FrameBatch::Sprite(batch) = self {
            Some(batch)
        } else {
            None
        }
    }

    fn shape_batch_mut(&mut self) -> Option<&'_ mut shape::Batch> {
        if let FrameBatch::Shape(batch) = self {
            Some(batch)
        } else {
            None
        }
    }
}

impl Frame {
    #[cfg_attr(
        feature = "tracing",
        instrument(name = "Frame::update", level = "trace", skip(self, event_receiver))
    )]
    pub fn update(&mut self, event_receiver: &flume::Receiver<SceneEvent>) {
        let elements = self.receiver.get_latest_frame(event_receiver);
        self.size = self.receiver.size;
        self.commands.clear();

        self.cache_glyphs(&elements);

        let mut referenced_texture_ids = HashSet::new();

        let mut current_texture_id: Option<u64> = None;
        let mut current_batch: Option<FrameBatch> = None;
        for element in &elements {
            match element {
                Element::Sprite {
                    sprite: sprite_handle,
                    clip,
                } => {
                    let sprite = sprite_handle.data.clone();
                    let texture = &sprite.source.texture;

                    if current_texture_id.is_none()
                        || current_texture_id.as_ref().unwrap() != &texture.id
                        || current_batch.is_none()
                        || !current_batch.as_ref().unwrap().is_sprite()
                        || current_batch
                            .as_ref()
                            .unwrap()
                            .sprite_batch()
                            .unwrap()
                            .clipping_rect
                            != *clip
                    {
                        self.commit_batch(current_batch);
                        current_texture_id = Some(texture.id);
                        referenced_texture_ids.insert(texture.id);

                        // Load the texture if needed
                        #[allow(clippy::map_entry)]
                        if !self.textures.contains_key(&texture.id) {
                            self.textures
                                .insert(texture.id, sprite.source.texture.clone());
                            self.commands
                                .push(FrameCommand::LoadTexture(sprite.source.texture.clone()));
                        }

                        current_batch = Some(FrameBatch::Sprite(sprite::Batch::new(
                            texture.id,
                            texture.size(),
                            *clip,
                        )));
                    }

                    let current_batch = current_batch.as_mut().unwrap().sprite_batch_mut().unwrap();
                    current_batch.sprites.push(sprite_handle.clone());
                }
                Element::Text { span, clip } => {
                    current_batch = self.commit_batch(current_batch);
                    self.commands.push(FrameCommand::DrawText {
                        text: span.clone(),
                        clip: *clip,
                    });
                }
                Element::Shape(shape) => {
                    if current_batch.is_some() && !current_batch.as_ref().unwrap().is_shape() {
                        current_batch = self.commit_batch(current_batch);
                    }

                    if current_batch.is_none() {
                        current_batch = Some(FrameBatch::Shape(shape::Batch::default()));
                    }

                    let current_batch = current_batch.as_mut().unwrap().shape_batch_mut().unwrap();
                    current_batch.add(shape.clone());
                }
            }
        }

        self.commit_batch(current_batch);

        let dead_texture_ids = self
            .textures
            .keys()
            .filter(|id| !referenced_texture_ids.contains(id))
            .cloned()
            .collect::<Vec<_>>();
        for id in dead_texture_ids {
            self.textures.remove(&id);
        }
    }

    fn commit_batch(&mut self, batch: Option<FrameBatch>) -> Option<FrameBatch> {
        if let Some(batch) = batch {
            match batch {
                FrameBatch::Sprite(batch) => self.commands.push(FrameCommand::DrawBatch(batch)),
                FrameBatch::Shape(batch) => self.commands.push(FrameCommand::DrawShapes(batch)),
            }
        }

        None
    }

    fn cache_glyphs(&mut self, elements: &[Element]) {
        let mut referenced_fonts = HashSet::new();
        for text in elements
            .iter()
            .map(|e| match &e {
                Element::Text { span, .. } => Some(span),
                _ => None,
            })
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
        {
            referenced_fonts.insert(text.data.font.id());

            for glyph_info in text.data.glyphs.iter() {
                let loaded_font = self
                    .fonts
                    .entry(text.data.font.id())
                    .or_insert_with(|| LoadedFont::new(&text.data.font));
                loaded_font.cache.queue_glyph(0, glyph_info.glyph.clone());
            }
        }

        let fonts_to_remove = self
            .fonts
            .keys()
            .filter(|id| !referenced_fonts.contains(id))
            .cloned()
            .collect::<Vec<_>>();
        for id in fonts_to_remove {
            self.fonts.remove(&id);
        }

        let mut updates = Vec::new();
        for font in self.fonts.values_mut() {
            let font_id = font.font.id();
            font.cache
                .cache_queued(|rect, data| {
                    updates.push(FontUpdate {
                        font_id,
                        rect,
                        data: data.to_vec(),
                    })
                })
                .expect("Error caching font"); // TODO Change this to a graceful
                                               // failure that
                                               // spams the console but doesn't
                                               // crash
        }
        self.pending_font_updates.extend(updates);
    }
}

#[derive(Debug)]
pub(crate) enum FrameCommand {
    LoadTexture(Texture),
    DrawBatch(sprite::Batch),
    DrawShapes(shape::Batch),
    DrawText {
        text: PreparedSpan,
        clip: Option<Rect<u32, Raw>>,
    },
}
