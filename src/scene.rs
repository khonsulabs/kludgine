use super::{
    math::{Point, Rect, Size},
    timing::Moment,
    KludgineHandle, KludgineResult,
};
use image::{DynamicImage, RgbaImage};
use rgx::core::BindingGroup;

use crossbeam::atomic::AtomicCell;
use lazy_static::lazy_static;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::Duration,
};
use winit::event::VirtualKeyCode;

use rusttype::gpu_cache;
pub(crate) enum Element {
    Sprite(Sprite),
    Text(Text),
}

pub struct Scene {
    pub pressed_keys: HashSet<VirtualKeyCode>,
    pub(crate) scale_factor: f32,
    pub(crate) size: Size,
    pub(crate) elements: Vec<Element>,
    now: Option<Moment>,
    elapsed: Option<Duration>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            scale_factor: 1.0,
            size: Size::default(),
            pressed_keys: HashSet::new(),
            now: None,
            elapsed: None,
            elements: Vec::new(),
        }
    }

    pub(crate) fn start_frame(&mut self) {
        let last_start = self.now;
        self.now = Some(Moment::now());
        self.elapsed = match last_start {
            Some(last_start) => self.now().checked_duration_since(&last_start),
            None => None,
        };
        self.elements.clear();
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn now(&self) -> Moment {
        self.now.expect("now() called without starting a frame")
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.elapsed
    }

    pub fn is_initial_frame(&self) -> bool {
        self.elapsed.is_none()
    }

    pub fn render_sprite_at(&mut self, source_sprite: &SourceSprite, location: Point) {
        let (w, h) = {
            let source = source_sprite
                .handle
                .read()
                .expect("Error locking source_sprite");
            (source.location.width(), source.location.height())
        };
        self.elements.push(Element::Sprite(Sprite::new(
            Rect::sized(location.x, location.y, w, h),
            source_sprite.clone(),
        )));
    }

    pub fn render_text_at<S: Into<String>>(
        &mut self,
        text: S,
        font: &Font,
        size: f32,
        location: Point,
        max_width: Option<f32>,
    ) {
        self.elements.push(Element::Text(Text {
            handle: KludgineHandle::new(TextData {
                font: font.clone(),
                text: text.into(),
                size,
                location,
                max_width,
                positioned_glyphs: None,
            }),
        }));
    }

    // pub fn get(&self, id: Entity) -> Option<Mesh> {
    //     match self.world.get_component::<MeshHandle>(id) {
    //         Some(handle) => Some(Mesh {
    //             id,
    //             handle: handle.as_ref().clone(),
    //         }),
    //         None => None,
    //     }
    // }

    // pub fn cached_mesh<S: Into<String>, F: FnOnce(&mut Scene2d) -> KludgineResult<Mesh>>(
    //     &mut self,
    //     name: S,
    //     initializer: F,
    // ) -> KludgineResult<Mesh> {
    //     let name = name.into();
    //     match self.lazy_mesh_cache.get(&name) {
    //         Some(mesh) => Ok(mesh.clone()),
    //         None => {
    //             let new_mesh = initializer(self)?;
    //             self.lazy_mesh_cache.insert(name, new_mesh.clone());
    //             Ok(new_mesh)
    //         }
    //     }
    // }

    // pub fn create_mesh<M: Into<Material>>(&mut self, shape: Shape, material: M) -> Mesh {
    //     let material = material.into();
    //     let storage = KludgineHandle::wrap(MeshStorage {
    //         shape,
    //         material,
    //         angle: Rad(0.0),
    //         scale: 1.0,
    //         position: Point2d::new(0.0, 0.0),
    //         children: HashMap::new(),
    //     });
    //     let handle = MeshHandle { storage };
    //     let id = self.world.insert((), vec![(handle.clone(),)])[0];
    //     Mesh { id, handle }
    // }
}

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

        self.size = scene.size;

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
                        if let Some(current_batch) = current_batch {
                            self.commands
                                .push(FrameCommand::DrawBatch(KludgineHandle::new(current_batch)));
                        }
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
                    let text_data = text
                        .handle
                        .read()
                        .expect("Error locking text for updating frame");
                    let font = text_data
                        .font
                        .handle
                        .read()
                        .expect("Error locking font for updating frame");
                    // TODO current_batch = None; -- probably refactor, "finishing" a batch needs to be done here too
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

        if let Some(current_batch) = current_batch {
            self.commands
                .push(FrameCommand::DrawBatch(KludgineHandle::new(current_batch)));
        }

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
    DrawText { text: Text, loaded_font: LoadedFont },
}

lazy_static! {
    static ref GLOBAL_ID_CELL: AtomicCell<u64> = { AtomicCell::new(0) };
}

#[derive(Clone)]
pub struct Texture {
    pub(crate) handle: KludgineHandle<TextureData>,
}

pub(crate) struct TextureData {
    pub id: u64,
    pub image: RgbaImage,
}

impl Texture {
    pub fn new(image: DynamicImage) -> Self {
        let image = image.to_rgba();
        let id = GLOBAL_ID_CELL.fetch_add(1);
        Self {
            handle: KludgineHandle::new(TextureData { id, image }),
        }
    }

    pub fn load<P: AsRef<Path>>(from_path: P) -> KludgineResult<Self> {
        let img = image::open(from_path)?;

        Ok(Self::new(img))
    }
}

#[derive(Clone)]
pub struct LoadedTexture {
    pub(crate) handle: KludgineHandle<LoadedTextureData>,
}

pub(crate) struct LoadedTextureData {
    pub texture: Texture,
    pub binding: Option<BindingGroup>,
}

impl LoadedTexture {
    pub fn new(texture: &Texture) -> Self {
        LoadedTexture {
            handle: KludgineHandle::new(LoadedTextureData {
                texture: texture.clone(),
                binding: None,
            }),
        }
    }
}

#[derive(Clone)]
pub struct SourceSprite {
    pub(crate) handle: KludgineHandle<SourceSpriteData>,
}

pub(crate) struct SourceSpriteData {
    pub location: Rect,
    pub texture: Texture,
}

impl SourceSprite {
    pub fn new(location: Rect, texture: Texture) -> Self {
        SourceSprite {
            handle: KludgineHandle::new(SourceSpriteData { location, texture }),
        }
    }

    pub fn entire_texture(texture: Texture) -> Self {
        let (w, h) = {
            let texture = texture.handle.read().expect("Error reading source sprice");
            (texture.image.width() as f32, texture.image.height() as f32)
        };
        Self::new(Rect::sized(0.0, 0.0, w, h), texture)
    }
}

#[derive(Clone)]
pub struct Sprite {
    pub(crate) handle: KludgineHandle<SpriteData>,
}

impl Sprite {
    pub fn new(render_at: Rect, source: SourceSprite) -> Self {
        Self {
            handle: KludgineHandle::new(SpriteData { render_at, source }),
        }
    }
}

pub(crate) struct SpriteData {
    pub render_at: Rect,
    pub source: SourceSprite,
}

pub(crate) struct SpriteBatch {
    pub loaded_texture: LoadedTexture,
    pub sprites: Vec<Sprite>,
}

impl SpriteBatch {
    pub fn new(loaded_texture: LoadedTexture) -> Self {
        SpriteBatch {
            loaded_texture,
            sprites: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct Font {
    pub(crate) handle: KludgineHandle<FontData>,
}

impl Font {
    pub fn try_from_bytes(bytes: &'static [u8]) -> Option<Font> {
        let font = rusttype::Font::try_from_bytes(bytes)?;
        Some(Font {
            handle: KludgineHandle::new(FontData {
                font,
                id: GLOBAL_ID_CELL.fetch_add(1),
            }),
        })
    }
}

pub(crate) struct FontData {
    pub(crate) id: u64,
    pub(crate) font: rusttype::Font<'static>,
}

#[derive(Clone)]
pub(crate) struct LoadedFont {
    pub handle: KludgineHandle<LoadedFontData>,
}

impl LoadedFont {
    pub fn new(font: &Font) -> Self {
        Self {
            handle: KludgineHandle::new(LoadedFontData {
                font: font.clone(),
                cache: gpu_cache::Cache::builder().dimensions(512, 512).build(),
                binding: None,
                texture: None,
            }),
        }
    }
}

pub(crate) struct LoadedFontData {
    pub font: Font,
    pub cache: gpu_cache::Cache<'static>,
    pub(crate) binding: Option<BindingGroup>,
    pub(crate) texture: Option<rgx::core::Texture>,
}

#[derive(Clone)]
pub struct Text {
    pub(crate) handle: KludgineHandle<TextData>,
}

pub struct TextData {
    pub font: Font,
    pub size: f32,
    pub text: String,
    pub location: Point,
    pub max_width: Option<f32>,
    pub positioned_glyphs: Option<Vec<rusttype::PositionedGlyph<'static>>>,
}
