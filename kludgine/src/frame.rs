use super::{
    math::Size,
    scene::{Element, Scene},
    sprite::SpriteBatch,
    text::{LoadedFont, RenderedSpan},
    texture::LoadedTexture,
    timing::Moment,
    KludgineHandle,
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

impl Frame {
    pub fn update(&mut self, scene: &Scene) {
        self.started_at = Some(scene.now());
        self.commands.clear();

        self.cache_glyphs(scene);

        self.size = scene.internal_size();

        let mut referenced_texture_ids = HashSet::new();

        let mut current_texture_id: Option<u64> = None;
        let mut current_batch: Option<SpriteBatch> = None;
        for element in scene.elements.iter() {
            match element {
                Element::Sprite(sprite_handle) => {
                    let sprite = sprite_handle
                        .handle
                        .read()
                        .expect("Error locking sprite for update");
                    let source = sprite
                        .source
                        .handle
                        .read()
                        .expect("Error locking source for update");
                    let texture = source
                        .texture
                        .handle
                        .read()
                        .expect("Error locking texture for update");

                    if current_texture_id.is_none()
                        || current_texture_id.as_ref().unwrap() != &texture.id
                    {
                        self.commit_batch(current_batch);
                        current_texture_id = Some(texture.id);
                        referenced_texture_ids.insert(texture.id);

                        // Load the texture if needed
                        let loaded_texture_handle = self
                            .textures
                            .entry(texture.id)
                            .or_insert_with(|| LoadedTexture::new(&source.texture));
                        let loaded_texture = loaded_texture_handle
                            .handle
                            .read()
                            .expect("Error locking loaded_texture");
                        if loaded_texture.binding.is_none() {
                            self.commands
                                .push(FrameCommand::LoadTexture(loaded_texture_handle.clone()));
                        }

                        current_batch = Some(SpriteBatch::new(loaded_texture_handle.clone()));
                    }

                    let current_batch = current_batch.as_mut().unwrap();
                    current_batch.sprites.push(sprite_handle.clone());
                }
                Element::Text(text) => {
                    current_batch = self.commit_batch(current_batch);
                    let text_data = text
                        .handle
                        .read()
                        .expect("Error locking text for updating frame");
                    let font = text_data
                        .font
                        .handle
                        .read()
                        .expect("Error locking font for updating frame");
                    let loaded_font = self
                        .fonts
                        .get(&font.id)
                        .expect("Text being drawn without font being loaded");
                    self.commands.push(FrameCommand::DrawText {
                        text: text.clone(),
                        loaded_font: loaded_font.clone(),
                    });
                }
            }
        }

        self.commit_batch(current_batch);

        let dead_texture_ids = self
            .textures
            .keys()
            .filter(|id| !referenced_texture_ids.contains(id))
            .map(|id| *id)
            .collect::<Vec<_>>();
        for id in dead_texture_ids {
            self.textures.remove(&id);
        }

        self.updated_at = Some(Moment::now());
    }

    fn commit_batch(&mut self, batch: Option<SpriteBatch>) -> Option<SpriteBatch> {
        if let Some(batch) = batch {
            self.commands
                .push(FrameCommand::DrawBatch(KludgineHandle::new(batch)));
        }

        None
    }

    fn cache_glyphs(&mut self, scene: &Scene) {
        let mut referenced_fonts = HashSet::new();
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
            let mut text = text.handle.write().expect("Error locking text for caching");
            if text.positioned_glyphs.is_none() {
                text.positioned_glyphs = Some({
                    let font = text
                        .font
                        .handle
                        .read()
                        .expect("Error locking font for caching");
                    referenced_fonts.insert(font.id);

                    let mut result = Vec::new();

                    let scale = rusttype::Scale::uniform(text.size);
                    let v_metrics = font.font.v_metrics(scale);
                    let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
                    let mut caret = rusttype::point(0.0, v_metrics.ascent);
                    let mut last_glyph_id = None;
                    for c in text.text.chars() {
                        if c.is_control() {
                            match c {
                                '\r' => {
                                    caret = rusttype::point(0.0, caret.y + advance_height);
                                }
                                '\n' => {}
                                _ => {}
                            }
                            continue;
                        }
                        let base_glyph = font.font.glyph(c);
                        if let Some(id) = last_glyph_id.take() {
                            caret.x += font.font.pair_kerning(scale, id, base_glyph.id());
                        }
                        last_glyph_id = Some(base_glyph.id());
                        let mut glyph = base_glyph.scaled(scale).positioned(caret);
                        if let Some(width) = text.max_width {
                            if let Some(bb) = glyph.pixel_bounding_box() {
                                if bb.max.x > width as i32 {
                                    caret = rusttype::point(0.0, caret.y + advance_height);
                                    glyph.set_position(caret);
                                    last_glyph_id = None;
                                }
                            }
                        }
                        caret.x += glyph.unpositioned().h_metrics().advance_width;
                        result.push(glyph);
                    }

                    result
                });
            }

            for glpyh in text.positioned_glyphs.as_ref().unwrap().iter() {
                let font = text
                    .font
                    .handle
                    .read()
                    .expect("Error locking font for caching");

                let loaded_font = self
                    .fonts
                    .entry(font.id)
                    .or_insert_with(|| LoadedFont::new(&text.font));
                let mut loaded_font_data = loaded_font
                    .handle
                    .write()
                    .expect("Error locking loaded font for writing");
                loaded_font_data.cache.queue_glyph(0, glpyh.clone());
            }
        }

        let fonts_to_remove = self
            .fonts
            .keys()
            .filter(|id| !referenced_fonts.contains(id))
            .map(|id| *id)
            .collect::<Vec<_>>();
        for id in fonts_to_remove {
            self.fonts.remove(&id);
        }

        for font in self.fonts.values().map(|f| f.clone()).collect::<Vec<_>>() {
            let mut loaded_font_data = font
                .handle
                .write()
                .expect("Error locking loaded font for writing");
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
    DrawBatch(KludgineHandle<SpriteBatch>),
    DrawText {
        text: RenderedSpan,
        loaded_font: LoadedFont,
    },
}
