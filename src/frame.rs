use super::{
    math::Size,
    scene::{Element, Scene},
    shape::{self},
    sprite::SpriteBatch,
    text::{font::LoadedFont, prepared::PreparedSpan},
    texture::LoadedTexture,
    timing::Moment,
};
use std::collections::{HashMap, HashSet};
#[derive(Default)]
pub(crate) struct Frame {
    pub started_at: Option<Moment>,
    pub updated_at: Option<Moment>,
    pub size: Size,
    pub commands: Vec<FrameCommand>,
    pub(crate) textures: HashMap<u64, LoadedTexture>,
    pub(crate) fonts: HashMap<u64, LoadedFont>,
    pub(crate) pending_font_updates: Vec<FontUpdate>,
}

pub(crate) struct FontUpdate {
    pub font: LoadedFont,
    pub rect: rusttype::Rect<u32>,
    pub data: Vec<u8>,
}

enum FrameBatch {
    Sprite(SpriteBatch),
    Shape(shape::Batch),
}

impl FrameBatch {
    fn is_shape(&self) -> bool {
        if let Self::Shape(_) = self {
            true
        } else {
            false
        }
    }

    fn is_sprite(&self) -> bool {
        !self.is_shape()
    }

    fn sprite_batch(&mut self) -> Option<&'_ mut SpriteBatch> {
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

        self.size = scene.internal_size().await.to_f32();

        let mut referenced_texture_ids = HashSet::new();

        let mut current_texture_id: Option<u64> = None;
        let mut current_batch: Option<FrameBatch> = None;
        let scene = scene.data.read().await;
        for element in scene.elements.iter() {
            match element {
                Element::Sprite(sprite_handle) => {
                    let sprite = sprite_handle.handle.read().await;
                    let source = sprite.source.handle.read().await;
                    let texture = source.texture.handle.read().await;

                    if current_texture_id.is_none()
                        || current_texture_id.as_ref().unwrap() != &texture.id
                        || current_batch.is_none()
                        || !current_batch.as_ref().unwrap().is_sprite()
                    {
                        self.commit_batch(current_batch);
                        current_texture_id = Some(texture.id);
                        referenced_texture_ids.insert(texture.id);

                        // Load the texture if needed
                        let loaded_texture_handle = self
                            .textures
                            .entry(texture.id)
                            .or_insert_with(|| LoadedTexture::new(&source.texture));
                        let loaded_texture = loaded_texture_handle.handle.read().await;
                        if loaded_texture.binding.is_none() {
                            self.commands
                                .push(FrameCommand::LoadTexture(loaded_texture_handle.clone()));
                        }

                        current_batch = Some(FrameBatch::Sprite(SpriteBatch::new(
                            loaded_texture_handle.clone(),
                        )));
                    }

                    let current_batch = current_batch.as_mut().unwrap().sprite_batch().unwrap();
                    current_batch.sprites.push(sprite_handle.clone());
                }
                Element::Text(text) => {
                    current_batch = self.commit_batch(current_batch);
                    let text_data = text.handle.read().await;
                    let font = text_data.font.handle.read().await;
                    let loaded_font = self
                        .fonts
                        .get(&font.id)
                        .expect("Text being drawn without font being loaded");
                    self.commands.push(FrameCommand::DrawText {
                        text: text.clone(),
                        loaded_font: loaded_font.clone(),
                    });
                }
                Element::Shape(shape) => {
                    if current_batch.is_some() && !current_batch.as_ref().unwrap().is_shape() {
                        current_batch = self.commit_batch(current_batch);
                    }

                    if current_batch.is_none() {
                        current_batch = Some(FrameBatch::Shape(shape::Batch::new()));
                    }

                    let current_batch = current_batch.as_mut().unwrap().shape_batch().unwrap();
                    current_batch.add(shape.clone()); // TODO clone? Can't we own the scene elements at this point?
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

        self.updated_at = Some(Moment::now());
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
            let text = text.handle.read().await;
            referenced_fonts.insert(text.font.id().await);

            for glpyh in text.positioned_glyphs.iter() {
                let font = text.font.handle.read().await;

                let loaded_font = self
                    .fonts
                    .entry(font.id)
                    .or_insert_with(|| LoadedFont::new(&text.font));
                let mut loaded_font_data = loaded_font.handle.write().await;
                loaded_font_data.cache.queue_glyph(0, glpyh.clone());
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

        for font in self.fonts.values().cloned().collect::<Vec<_>>() {
            let mut loaded_font_data = font.handle.write().await;
            loaded_font_data
                .cache
                .cache_queued(|rect, data| {
                    self.pending_font_updates.push(FontUpdate {
                        font: font.clone(),
                        rect,
                        data: data.to_vec(),
                    })
                })
                .expect("Error caching font");
        }
    }
}

pub(crate) enum FrameCommand {
    LoadTexture(LoadedTexture),
    DrawBatch(SpriteBatch),
    DrawShapes(shape::Batch),
    DrawText {
        text: PreparedSpan,
        loaded_font: LoadedFont,
    },
}
