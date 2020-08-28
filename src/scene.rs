use crate::{
    math::{Pixels, Point, Points, Size},
    shape::Shape,
    sprite::RenderedSprite,
    style::Weight,
    text::{font::Font, prepared::PreparedSpan},
    theme::{Minimal, Theme},
    KludgineError, KludgineHandle, KludgineResult,
};
use platforms::target::{OS, TARGET_OS};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};
use winit::event::VirtualKeyCode;

pub(crate) enum Element {
    Sprite(RenderedSprite),
    Text(PreparedSpan),
    Shape(Shape),
}

#[derive(Clone)]
pub enum SceneTarget {
    Scene(Scene),
    Camera {
        origin: Point<Points>,
        zoom: f32,
        scene: Scene,
    },
}

impl SceneTarget {
    pub async fn size(&self) -> Size<Points> {
        let size = match &self {
            SceneTarget::Scene(scene) => scene.size(),
            SceneTarget::Camera { scene, .. } => scene.size(),
        }
        .await;
        let effective_scale_factor = self.effective_scale_factor().await;
        size.to_points(effective_scale_factor)
    }

    pub async fn effective_scale_factor(&self) -> f32 {
        match &self {
            SceneTarget::Scene(scene) => scene.scale_factor().await,
            SceneTarget::Camera { scene, zoom, .. } => scene.scale_factor().await * zoom,
        }
    }

    async fn scene(&self) -> async_rwlock::RwLockReadGuard<'_, SceneData> {
        match self {
            SceneTarget::Scene(scene) => scene.data.read().await,
            SceneTarget::Camera { scene, .. } => scene.data.read().await,
        }
    }

    async fn scene_mut(&self) -> async_rwlock::RwLockWriteGuard<'_, SceneData> {
        match self {
            SceneTarget::Scene(scene) => scene.data.write().await,
            SceneTarget::Camera { scene, .. } => scene.data.write().await,
        }
    }

    pub(crate) async fn push_element(&self, element: Element) {
        self.scene_mut().await.elements.push(element);
    }

    pub async fn draw_shape(&self, shape: Shape) {
        let shape = match shape {
            Shape::Rectangle(rectangle, zdepth, rotation, stroke, fill) => {
                let effective_scale = self.effective_scale_factor().await;
                let p1 = self
                    .user_to_device_point(Point::new(rectangle.x1.into(), rectangle.y1.into()))
                    .await
                    .to_pixels(effective_scale);
                let p2 = self
                    .user_to_device_point(Point::new(rectangle.x2.into(), rectangle.y2.into()))
                    .await
                    .to_pixels(effective_scale);

                Shape::Rectangle(
                    rgx::rect::Rect::new(
                        p1.x.to_f32(),
                        p1.y.to_f32(),
                        p2.x.to_f32(),
                        p2.y.to_f32(),
                    ),
                    zdepth,
                    rotation,
                    stroke,
                    fill,
                )
            }
            Shape::Line(line, zdepth, rotation, stroke) => {
                let effective_scale = self.effective_scale_factor().await;
                let p1 = self
                    .user_to_device_point(Point::new(line.p1.x.into(), line.p1.y.into()))
                    .await
                    .to_pixels(effective_scale);
                let p2 = self
                    .user_to_device_point(Point::new(line.p2.x.into(), line.p2.y.into()))
                    .await
                    .to_pixels(effective_scale);
                Shape::Line(
                    rgx::kit::shape2d::Line {
                        p1: rgx::math::Point2::new(p1.x.to_f32(), p1.y.to_f32()),
                        p2: rgx::math::Point2::new(p2.x.to_f32(), p2.y.to_f32()),
                    },
                    zdepth,
                    rotation,
                    stroke,
                )
            }
            Shape::Circle(..) => {
                todo!("rgx needs to expose the fields on shape2d::Circle to be pub, can't make this code work otherwise https://github.com/cloudhead/rgx/issues/25")
                // let position = self
                //     .user_to_device_point(circle.position.into())
                //     .await
                //     .into();
                // Shape::Circle(
                //     rgx::kit::shape2d::Circle {
                //         position,
                //         radius: circle.radius,
                //         sides: circle.sides,
                //     },
                //     zdepth,
                //     stroke,
                //     fill,
                // )
            }
        };
        self.scene_mut().await.elements.push(Element::Shape(shape));
    }

    pub fn set_camera(&self, zoom: f32, look_at: Point<Points>) -> SceneTarget {
        let origin = Point::new(-look_at.x, -look_at.y);
        match self {
            SceneTarget::Scene(scene) => SceneTarget::Camera {
                scene: scene.clone(),
                zoom,
                origin,
            },
            SceneTarget::Camera { scene, .. } => SceneTarget::Camera {
                scene: scene.clone(),
                zoom,
                origin,
            },
        }
    }

    pub fn set_zoom(&self, zoom: f32) -> SceneTarget {
        match self {
            SceneTarget::Scene(scene) => SceneTarget::Camera {
                scene: scene.clone(),
                zoom,
                origin: Point::default(),
            },
            SceneTarget::Camera { scene, origin, .. } => SceneTarget::Camera {
                scene: scene.clone(),
                zoom,
                origin: *origin,
            },
        }
    }

    pub(crate) async fn user_to_device_point(&self, point: Point<Points>) -> Point<Points> {
        Point::new(
            point.x + self.origin().x,
            self.size().await.height - (point.y + self.origin().y),
        )
    }

    pub async fn lookup_font(&self, family: &str, weight: Weight) -> KludgineResult<Font> {
        match &self {
            SceneTarget::Scene(scene) => scene.lookup_font(family, weight).await,
            SceneTarget::Camera { scene, .. } => scene.lookup_font(family, weight).await,
        }
    }

    pub async fn theme(&self) -> Arc<Box<dyn Theme>> {
        self.scene().await.theme.clone()
    }

    pub fn origin(&self) -> Point<Points> {
        match &self {
            SceneTarget::Scene(_) => Point::default(),
            SceneTarget::Camera { origin, .. } => *origin,
        }
    }

    pub fn zoom(&self) -> f32 {
        match &self {
            SceneTarget::Scene(_) => 1.0,
            SceneTarget::Camera { zoom, .. } => *zoom,
        }
    }

    pub async fn elapsed(&self) -> Option<Duration> {
        self.scene().await.elapsed
    }

    pub async fn pressed_keys(&self) -> HashSet<VirtualKeyCode> {
        self.scene().await.pressed_keys.clone()
    }

    pub async fn key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.scene().await.pressed_keys.contains(&key)
    }

    pub async fn register_font(&mut self, font: &Font) {
        let scene = match self {
            SceneTarget::Scene(scene) => scene,
            SceneTarget::Camera { scene, .. } => scene,
        };

        scene.register_font(font).await;
    }
}

#[derive(Clone)]
pub struct Scene {
    pub(crate) data: KludgineHandle<SceneData>,
}

pub(crate) struct SceneData {
    pub pressed_keys: HashSet<VirtualKeyCode>,
    scale_factor: f32,
    size: Size<Pixels>,
    pub(crate) elements: Vec<Element>,
    now: Option<Instant>,
    elapsed: Option<Duration>,
    fonts: HashMap<String, Vec<Font>>,
    theme: Arc<Box<dyn Theme>>,
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

impl Default for Scene {
    fn default() -> Self {
        Self {
            data: KludgineHandle::new(SceneData {
                scale_factor: 1.0,
                size: Size::default(),
                pressed_keys: HashSet::new(),
                now: None,
                elapsed: None,
                elements: Vec::new(),
                fonts: HashMap::new(),
                theme: Arc::new(Box::new(Minimal::default())),
            }),
        }
    }
}

impl Scene {
    pub(crate) async fn set_internal_size(&self, size: Size<Pixels>) {
        let mut scene = self.data.write().await;
        scene.size = size;
    }

    pub(crate) async fn internal_size(&self) -> Size<Pixels> {
        let scene = self.data.read().await;
        scene.size
    }

    pub(crate) async fn set_scale_factor(&mut self, scale_factor: f32) {
        let mut scene = self.data.write().await;
        scene.scale_factor = scale_factor;
    }

    pub async fn scale_factor(&self) -> f32 {
        let scene = self.data.read().await;
        scene.scale_factor
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

    pub async fn size(&self) -> Size<Pixels> {
        let scene = self.data.read().await;
        scene.size
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

    pub async fn register_font(&mut self, font: &Font) {
        let family = font.family().await.expect("Unable to register VecFonts");
        let mut scene = self.data.write().await;
        scene
            .fonts
            .entry(family)
            .and_modify(|fonts| fonts.push(font.clone()))
            .or_insert_with(|| vec![font.clone()]);
    }

    pub(crate) async fn register_bundled_fonts(&mut self) {
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

    pub async fn lookup_font(&self, family: &str, weight: Weight) -> KludgineResult<Font> {
        let family = if family.eq_ignore_ascii_case("sans-serif") {
            "Roboto" // TODO Themes should resolve the font family name
        } else {
            family
        };
        let scene = self.data.read().await;
        match scene.fonts.get(family) {
            Some(fonts) => {
                let mut closest_font = None;
                let mut closest_weight = None;

                for font in fonts.iter() {
                    if font.weight().await == weight {
                        return Ok(font.clone());
                    } else {
                        let delta = (font.weight().await.to_number() as i32
                            - weight.to_number() as i32)
                            .abs();
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

    pub async fn theme(&self) -> Arc<Box<dyn Theme>> {
        let scene = self.data.read().await;
        scene.theme.clone()
    }
}
