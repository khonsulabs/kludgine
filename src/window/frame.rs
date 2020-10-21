use easygpu::transform::ScreenSpace;

use crate::{
    math::Size,
    scene::{Element, Scene},
    shape, sprite,
    text::{font::LoadedFont, prepared::PreparedSpan},
    texture::Texture,
};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};
#[derive(Default)]
pub(crate) struct Frame {
    pub started_at: Option<Instant>,
    pub updated_at: Option<Instant>,
    pub size: Size<f32, ScreenSpace>,
    pub commands: Vec<FrameCommand>,
    pub(crate) textures: HashMap<u64, Texture>,
    pub(crate) fonts: HashMap<u64, LoadedFont>,
    pub(crate) pending_font_updates: Vec<FontUpdate>,
}

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

    fn sprite_batch(&mut self) -> Option<&'_ mut sprite::Batch> {
        if let FrameBatch::Sprite(batch) = self {
            Some(batch)
        } else {
            None
        }
    }

    fn shape_batch(&mut self) -> Option<&'_ mut shape::Batch> {
        if let FrameBatch::Shape(batch) = self {
            Some(batch)
        } else {
            None
        }
    }
}

impl Frame {
    pub async fn update(&mut self, scene: &Scene) {
        self.started_at = Some(scene.now().await);
        self.commands.clear();

        self.cache_glyphs(scene).await;

        self.size = scene.internal_size().await.cast_unit();

        let mut referenced_texture_ids = HashSet::new();

        let mut current_texture_id: Option<u64> = None;
        let mut current_batch: Option<FrameBatch> = None;
        let scene = scene.data.read().await;
        for element in scene.elements.iter() {
            match element {
                Element::Sprite(sprite_handle) => {
                    let sprite = sprite_handle.data.clone();
                    let texture = &sprite.source.texture;

                    if current_texture_id.is_none()
                        || current_texture_id.as_ref().unwrap() != &texture.id
                        || current_batch.is_none()
                        || !current_batch.as_ref().unwrap().is_sprite()
                    {
                        self.commit_batch(current_batch);
                        current_texture_id = Some(texture.id);
                        referenced_texture_ids.insert(texture.id);

                        // Load the texture if needed
                        if !self.textures.contains_key(&texture.id) {
                            self.textures
                                .insert(texture.id, sprite.source.texture.clone());
                            self.commands
                                .push(FrameCommand::LoadTexture(sprite.source.texture.clone()));
                        }

                        current_batch = Some(FrameBatch::Sprite(sprite::Batch::new(
                            texture.id,
                            texture.size(),
                        )));
                    }

                    let current_batch = current_batch.as_mut().unwrap().sprite_batch().unwrap();
                    current_batch.sprites.push(sprite_handle.clone());
                }
                Element::Text(text) => {
                    current_batch = self.commit_batch(current_batch);
                    self.commands
                        .push(FrameCommand::DrawText { text: text.clone() });
                }
                Element::Shape(shape) => {
                    if current_batch.is_some() && !current_batch.as_ref().unwrap().is_shape() {
                        current_batch = self.commit_batch(current_batch);
                    }

                    if current_batch.is_none() {
                        current_batch = Some(FrameBatch::Shape(shape::Batch::default()));
                    }

                    let current_batch = current_batch.as_mut().unwrap().shape_batch().unwrap();
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

        self.updated_at = Some(Instant::now());
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

    async fn cache_glyphs(&mut self, scene: &Scene) {
        let mut referenced_fonts = HashSet::new();
        let scene = scene.data.read().await;
        for text in scene
            .elements
            .iter()
            .map(|e| match &e {
                Element::Text(t) => Some(t),
                _ => None,
            })
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
        {
            referenced_fonts.insert(text.data.font.id().await);

            for glyph_info in text.data.glyphs.iter() {
                let font = text.data.font.handle.read().await;

                let loaded_font = self
                    .fonts
                    .entry(font.id)
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
            let font_id = font.font.id;
            font.cache
                .cache_queued(|rect, data| {
                    updates.push(FontUpdate {
                        font_id,
                        rect,
                        data: data.to_vec(),
                    })
                })
                .expect("Error caching font"); // TODO Change this to a graceful failure
        }
        self.pending_font_updates.extend(updates);
    }
}

pub(crate) enum FrameCommand {
    LoadTexture(Texture),
    DrawBatch(sprite::Batch),
    DrawShapes(shape::Batch),
    DrawText { text: PreparedSpan },
}
