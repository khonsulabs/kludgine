use crate::{
    math::{Raw, Scale, Scaled, ScreenScale, Size},
    shape::Shape,
    sprite::RenderedSprite,
    style::{
        theme::{SystemTheme, Theme},
        FontStyle, Weight,
    },
    text::{font::Font, prepared::PreparedSpan},
    Handle, KludgineError, KludgineResult,
};
use euclid::Rect;
use platforms::target::{OS, TARGET_OS};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};
use winit::event::VirtualKeyCode;

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

#[derive(Clone, Debug)]
pub struct Scene {
    pub(crate) data: Handle<SceneData>,
}

impl From<Scene> for Target {
    fn from(scene: Scene) -> Target {
        Self::Scene(scene)
    }
}

#[derive(Clone, Debug)]
pub enum Target {
    Scene(Scene),
    ClippedScene(Scene, Rect<u32, Raw>),
}

impl Target {
    pub fn clipping_rect(&self) -> Option<Rect<u32, Raw>> {
        match self {
            Self::Scene(_) => None,
            Self::ClippedScene(_, clip) => Some(*clip),
        }
    }

    pub fn scene(&self) -> &Scene {
        match self {
            Self::Scene(scene) => scene,
            Self::ClippedScene(scene, _) => scene,
        }
    }

    pub fn clipped_to(&self, new_clip: Rect<u32, Raw>) -> Self {
        Self::ClippedScene(
            self.scene().clone(),
            match self.clipping_rect() {
                Some(existing_clip) => existing_clip.union(&new_clip),
                None => new_clip,
            },
        )
    }
}

impl std::ops::Deref for Target {
    type Target = Scene;

    fn deref(&self) -> &Self::Target {
        self.scene()
    }
}

impl std::ops::DerefMut for Target {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Scene(scene) => scene,
            Self::ClippedScene(scene, ..) => scene,
        }
    }
}

#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub(crate) struct SceneData {
    pub pressed_keys: HashSet<VirtualKeyCode>,
    scale_factor: ScreenScale,
    size: Size<f32, Raw>,
    pub(crate) elements: Vec<Element>,
    now: Option<Instant>,
    elapsed: Option<Duration>,
    fonts: HashMap<String, Vec<Font>>,
    system_theme: SystemTheme,
    #[derivative(Debug = "ignore")]
    theme: Arc<Theme>,
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
    pub(crate) fn new(theme: Theme) -> Self {
        Self {
            data: Handle::new(SceneData {
                theme: Arc::new(theme),
                scale_factor: Scale::identity(),
                size: Size::default(),
                pressed_keys: HashSet::new(),
                now: None,
                elapsed: None,
                elements: Vec::new(),
                fonts: HashMap::new(),
                system_theme: SystemTheme::Light,
            }),
        }
    }

    pub async fn system_theme(&self) -> SystemTheme {
        let scene = self.data.read().await;
        scene.system_theme.clone()
    }

    pub(crate) async fn set_system_theme(&self, system_theme: SystemTheme) {
        let mut scene = self.data.write().await;
        scene.system_theme = system_theme;
    }

    pub(crate) async fn push_element(&self, element: Element) {
        let mut scene = self.data.write().await;
        scene.elements.push(element);
    }

    pub(crate) async fn set_internal_size(&self, size: Size<f32, Raw>) {
        let mut scene = self.data.write().await;
        scene.size = size;
    }

    pub(crate) async fn internal_size(&self) -> Size<f32, Raw> {
        let scene = self.data.read().await;
        scene.size
    }

    pub(crate) async fn set_scale_factor(&mut self, scale_factor: ScreenScale) {
        let mut scene = self.data.write().await;
        scene.scale_factor = scale_factor;
    }

    pub async fn scale_factor(&self) -> ScreenScale {
        let scene = self.data.read().await;
        scene.scale_factor
    }

    pub async fn keys_pressed(&self) -> HashSet<VirtualKeyCode> {
        let scene = self.data.read().await;
        scene.pressed_keys.clone()
    }

    pub async fn key_pressed(&self, key: VirtualKeyCode) -> bool {
        let scene = self.data.read().await;
        scene.pressed_keys.contains(&key)
    }

    pub async fn any_key_pressed(&self, keys: &[VirtualKeyCode]) -> bool {
        let scene = self.data.read().await;
        for key in keys {
            if scene.pressed_keys.contains(key) {
                return true;
            }
        }
        false
    }

    pub async fn modifiers_pressed(&self) -> Modifiers {
        let (control, alt, shift, os) = futures::join!(
            self.any_key_pressed(&[VirtualKeyCode::RControl, VirtualKeyCode::LControl]),
            self.any_key_pressed(&[VirtualKeyCode::RAlt, VirtualKeyCode::LAlt]),
            self.any_key_pressed(&[VirtualKeyCode::LShift, VirtualKeyCode::RShift]),
            self.any_key_pressed(&[VirtualKeyCode::RWin, VirtualKeyCode::LWin])
        );
        Modifiers {
            control,
            alt,
            shift,
            os,
        }
    }

    pub(crate) async fn start_frame(&mut self) {
        let mut scene = self.data.write().await;
        let last_start = scene.now;
        scene.now = Some(Instant::now());
        scene.elapsed = match last_start {
            Some(last_start) => scene.now.unwrap().checked_duration_since(last_start),
            None => None,
        };
        scene.elements.clear();
    }

    pub async fn size(&self) -> Size<f32, Scaled> {
        let scene = self.data.read().await;
        scene.size / scene.scale_factor
    }

    pub async fn now(&self) -> Instant {
        let scene = self.data.read().await;
        scene.now.expect("now() called without starting a frame")
    }

    pub async fn elapsed(&self) -> Option<Duration> {
        let scene = self.data.read().await;
        scene.elapsed
    }

    pub async fn is_initial_frame(&self) -> bool {
        let scene = self.data.read().await;
        scene.elapsed.is_none()
    }

    pub async fn register_font(&self, font: &Font) {
        let family = font.family().await.expect("Unable to register VecFonts");
        let mut scene = self.data.write().await;
        scene
            .fonts
            .entry(family)
            .and_modify(|fonts| fonts.push(font.clone()))
            .or_insert_with(|| vec![font.clone()]);
    }

    #[cfg(feature = "bundled-fonts-enabled")]
    pub(crate) async fn register_bundled_fonts(&self) {
        #[cfg(feature = "bundled-fonts-roboto")]
        {
            self.register_font(&crate::text::bundled_fonts::ROBOTO)
                .await;
            self.register_font(&crate::text::bundled_fonts::ROBOTO_ITALIC)
                .await;
            self.register_font(&crate::text::bundled_fonts::ROBOTO_BLACK)
                .await;
            self.register_font(&crate::text::bundled_fonts::ROBOTO_BLACK_ITALIC)
                .await;
            self.register_font(&crate::text::bundled_fonts::ROBOTO_BOLD)
                .await;
            self.register_font(&crate::text::bundled_fonts::ROBOTO_BOLD_ITALIC)
                .await;
            self.register_font(&crate::text::bundled_fonts::ROBOTO_LIGHT)
                .await;
            self.register_font(&crate::text::bundled_fonts::ROBOTO_LIGHT_ITALIC)
                .await;
            self.register_font(&crate::text::bundled_fonts::ROBOTO_MEDIUM)
                .await;
            self.register_font(&crate::text::bundled_fonts::ROBOTO_MEDIUM_ITALIC)
                .await;
            self.register_font(&crate::text::bundled_fonts::ROBOTO_THIN)
                .await;
            self.register_font(&crate::text::bundled_fonts::ROBOTO_THIN_ITALIC)
                .await;
        }
    }

    pub async fn lookup_font(
        &self,
        family: &str,
        weight: Weight,
        style: FontStyle,
    ) -> KludgineResult<Font> {
        let scene = self.data.read().await;
        let fonts = if family.eq_ignore_ascii_case("sans-serif") {
            let theme = self.theme().await;
            scene.fonts.get(&theme.default_font_family)
        } else {
            scene.fonts.get(family)
        };

        match fonts {
            Some(fonts) => {
                let mut closest_font = None;
                let mut closest_weight = None;

                for font in fonts.iter() {
                    let font_weight = font.weight().await;
                    let font_style = font.style().await;

                    if font_weight == weight && font_style == style {
                        return Ok(font.clone());
                    } else {
                        // If it's not the right style, we want to heavily penalize the score
                        // But if no font matches the style, we should pick the weight that matches
                        // best in another style.
                        let style_multiplier = if font_style == style { 1 } else { 10 };
                        let delta = (font.weight().await.to_number() as i32
                            - weight.to_number() as i32)
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

    pub async fn theme(&self) -> Arc<Theme> {
        let scene = self.data.read().await;
        scene.theme.clone()
    }
}
