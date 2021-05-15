use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use euclid::Rect;
use platforms::target::{OS, TARGET_OS};
use stylecs::{FontStyle, Weight};
use winit::{event::VirtualKeyCode, window::Theme};

use crate::{
    math::{Point, Raw, Scale, Scaled, ScreenScale, Size, Vector},
    shape::Shape,
    sprite::RenderedSprite,
    text::{font::Font, prepared::PreparedSpan},
    KludgineError, KludgineResult,
};

#[derive(Debug)]
pub(crate) enum Element {
    Sprite {
        sprite: RenderedSprite,
        clip: Option<Rect<u32, Raw>>,
    },
    Text {
        span: PreparedSpan,
        clip: Option<Rect<u32, Raw>>,
    },
    Shape(Shape<Raw>),
}

pub(crate) enum SceneEvent {
    Render(Element),
    EndFrame,
    BeginFrame { size: Size<f32, Raw> },
}

#[derive(Debug)]
pub struct Scene {
    pub pressed_keys: HashSet<VirtualKeyCode>,
    scale_factor: ScreenScale,
    size: Size<f32, Raw>,
    event_sender: flume::Sender<SceneEvent>,
    now: Option<Instant>,
    elapsed: Option<Duration>,
    fonts: HashMap<String, Vec<Font>>,
    system_theme: Theme,
}

impl From<Arc<Scene>> for Target {
    fn from(scene: Arc<Scene>) -> Target {
        Self {
            scene,
            clip: None,
            offset: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Target {
    pub scene: Arc<Scene>,
    pub clip: Option<Rect<u32, Raw>>,
    pub offset: Option<Vector<f32, Raw>>,
}

impl Target {
    pub fn clipped_to(&self, new_clip: Rect<u32, Raw>) -> Self {
        Self {
            scene: self.scene.clone(),
            clip: Some(match &self.clip {
                Some(existing_clip) => existing_clip.union(&new_clip),
                None => new_clip,
            }),
            offset: self.offset,
        }
    }

    pub fn offset_by(&self, delta: Vector<f32, Raw>) -> Self {
        Self {
            scene: self.scene.clone(),
            clip: self.clip,
            offset: Some(match self.offset {
                Some(offset) => offset + delta,
                None => delta,
            }),
        }
    }

    pub fn offset_point(&self, point: Point<f32, Scaled>) -> Point<f32, Scaled> {
        match self.offset {
            Some(offset) => point + offset / self.scale_factor(),
            None => point,
        }
    }

    pub fn offset_point_raw(&self, point: Point<f32, Raw>) -> Point<f32, Raw> {
        match self.offset {
            Some(offset) => point + offset,
            None => point,
        }
    }
}

impl std::ops::Deref for Target {
    type Target = Scene;

    fn deref(&self) -> &Self::Target {
        self.scene.as_ref()
    }
}

pub struct Modifiers {
    pub control: bool,
    pub alt: bool,
    pub os: bool,
    pub shift: bool,
}

impl Modifiers {
    pub fn primary_modifier(&self) -> bool {
        match TARGET_OS {
            OS::MacOS | OS::iOS => self.os,
            _ => self.control,
        }
    }

    pub fn command_key(&self) -> bool {
        match TARGET_OS {
            OS::MacOS | OS::iOS => self.os,
            _ => false,
        }
    }
}

impl Scene {
    pub(crate) fn new(event_sender: flume::Sender<SceneEvent>) -> Self {
        Self {
            event_sender,
            scale_factor: Scale::identity(),
            size: Size::default(),
            pressed_keys: HashSet::new(),
            now: None,
            elapsed: None,
            fonts: HashMap::new(),
            system_theme: Theme::Light,
        }
    }

    pub fn system_theme(&self) -> Theme {
        self.system_theme
    }

    pub(crate) fn set_system_theme(&mut self, system_theme: Theme) {
        self.system_theme = system_theme;
    }

    pub(crate) fn push_element(&self, element: Element) {
        let _ = self.event_sender.send(SceneEvent::Render(element));
    }

    pub(crate) fn set_internal_size(&mut self, size: Size<f32, Raw>) {
        self.size = size;
    }

    // pub(crate) async fn internal_size(&self) -> Size<f32, Raw> {
    //     let scene = self.data.read().await;
    //     scene.size
    // }

    pub(crate) fn set_scale_factor(&mut self, scale_factor: ScreenScale) {
        self.scale_factor = scale_factor;
    }

    pub fn scale_factor(&self) -> ScreenScale {
        self.scale_factor
    }

    pub fn keys_pressed(&self) -> HashSet<VirtualKeyCode> {
        self.pressed_keys.clone()
    }

    pub fn key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn any_key_pressed(&self, keys: &[VirtualKeyCode]) -> bool {
        for key in keys {
            if self.pressed_keys.contains(key) {
                return true;
            }
        }
        false
    }

    pub fn modifiers_pressed(&self) -> Modifiers {
        Modifiers {
            control: self.any_key_pressed(&[VirtualKeyCode::RControl, VirtualKeyCode::LControl]),
            alt: self.any_key_pressed(&[VirtualKeyCode::RAlt, VirtualKeyCode::LAlt]),
            shift: self.any_key_pressed(&[VirtualKeyCode::LShift, VirtualKeyCode::RShift]),
            os: self.any_key_pressed(&[VirtualKeyCode::RWin, VirtualKeyCode::LWin]),
        }
    }

    pub(crate) fn start_frame(&mut self) {
        let last_start = self.now;
        self.now = Some(Instant::now());
        self.elapsed = match last_start {
            Some(last_start) => self.now.unwrap().checked_duration_since(last_start),
            None => None,
        };
        let _ = self
            .event_sender
            .send(SceneEvent::BeginFrame { size: self.size });
    }

    pub(crate) fn end_frame(&self) {
        let _ = self.event_sender.send(SceneEvent::EndFrame);
    }

    pub fn size(&self) -> Size<f32, Scaled> {
        self.size / self.scale_factor
    }

    pub fn now(&self) -> Instant {
        self.now.expect("now() called without starting a frame")
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.elapsed
    }

    pub fn is_initial_frame(&self) -> bool {
        self.elapsed.is_none()
    }

    pub fn register_font(&mut self, font: &Font) {
        let family = font.family().expect("Unable to register VecFonts");
        self.fonts
            .entry(family)
            .and_modify(|fonts| fonts.push(font.clone()))
            .or_insert_with(|| vec![font.clone()]);
    }

    #[cfg(feature = "bundled-fonts-enabled")]
    pub(crate) fn register_bundled_fonts(&mut self) {
        #[cfg(feature = "bundled-fonts-roboto")]
        {
            self.register_font(&crate::text::bundled_fonts::ROBOTO);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_ITALIC);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_BLACK);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_BLACK_ITALIC);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_BOLD);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_BOLD_ITALIC);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_LIGHT);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_LIGHT_ITALIC);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_MEDIUM);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_MEDIUM_ITALIC);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_THIN);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_THIN_ITALIC);
        }
    }

    pub fn lookup_font(
        &self,
        family: &str,
        weight: Weight,
        style: FontStyle,
    ) -> KludgineResult<Font> {
        let fonts = self.fonts.get(family);

        match fonts {
            Some(fonts) => {
                let mut closest_font = None;
                let mut closest_weight = None;

                for font in fonts.iter() {
                    let font_weight = font.weight();
                    let font_style = font.style();

                    if font_weight == weight && font_style == style {
                        return Ok(font.clone());
                    } else {
                        // If it's not the right style, we want to heavily penalize the score
                        // But if no font matches the style, we should pick the weight that matches
                        // best in another style.
                        let style_multiplier = if font_style == style { 1 } else { 10 };
                        let delta = (font.weight().to_number() as i32 - weight.to_number() as i32)
                            .abs()
                            * style_multiplier;

                        if closest_weight.is_none() || closest_weight.unwrap() > delta {
                            closest_weight = Some(delta);
                            closest_font = Some(font.clone());
                        }
                    }
                }

                closest_font.ok_or_else(|| KludgineError::FontFamilyNotFound(family.to_owned()))
            }
            None => Err(KludgineError::FontFamilyNotFound(family.to_owned())),
        }
    }
}
