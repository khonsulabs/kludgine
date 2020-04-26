use super::{
    math::{Point, Size, Zeroable},
    sprite::RenderedSprite,
    style::Weight,
    text::{Font, PreparedSpan},
    timing::Moment,
    KludgineError, KludgineResult,
};
use platforms::target::{OS, TARGET_OS};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use winit::event::VirtualKeyCode;

pub(crate) enum Element {
    Sprite(RenderedSprite),
    Text(PreparedSpan),
}

pub struct Scene {
    pub pressed_keys: HashSet<VirtualKeyCode>,
    scale_factor: f32,
    origin: Point,
    zoom: f32,
    size: Size,
    pub(crate) elements: Vec<Element>,
    now: Option<Moment>,
    elapsed: Option<Duration>,
    fonts: HashMap<String, Vec<Font>>,
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
            origin: Point::zero(),
            zoom: 1.0,
            fonts: HashMap::new(),
        }
    }

    pub(crate) fn set_internal_size(&mut self, size: Size) {
        self.size = size;
    }

    pub(crate) fn internal_size(&self) -> Size {
        self.size
    }

    pub(crate) fn set_scale_factor(&mut self, scale_factor: f32) {
        self.scale_factor = scale_factor;
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom;
    }

    pub fn origin(&self) -> Point {
        self.origin
    }

    pub fn set_origin(&mut self, origin: Point) {
        self.origin = origin;
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

    pub(crate) fn effective_scale_factor(&self) -> f32 {
        self.scale_factor * self.zoom
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
        Size::new(
            self.size.width / self.effective_scale_factor(),
            self.size.height / self.effective_scale_factor(),
        )
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

    pub(crate) fn user_to_device_point<S>(&self, point: Point<S>) -> Point<S>
    where
        S: From<f32> + std::ops::Sub<Output = S> + std::ops::Add<Output = S>,
    {
        Point::new(
            point.x + Into::<S>::into(self.origin.x),
            Into::<S>::into(self.size().height) - (point.y + Into::<S>::into(self.origin.y)),
        )
    }

    pub fn register_font(&mut self, font: &Font) {
        let family = font.family().expect("Unable to register VecFonts");
        self.fonts
            .entry(family)
            .and_modify(|fonts| fonts.push(font.clone()))
            .or_insert_with(|| vec![font.clone()]);
    }

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

    pub fn lookup_font(&self, family: &str, weight: Weight) -> KludgineResult<Font> {
        let family = if family.eq_ignore_ascii_case("sans-serif") {
            "Roboto"
        } else {
            family
        };
        match self.fonts.get(family) {
            Some(fonts) => {
                let mut closest_font = None;
                let mut closest_weight = None;

                for font in fonts.iter() {
                    if font.weight() == weight {
                        return Ok(font.clone());
                    } else {
                        let delta =
                            (font.weight().to_number() as i32 - weight.to_number() as i32).abs();
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
