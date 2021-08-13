use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use figures::{DisplayScale, Displayable, One, Pixels, Points, Rectlike, SizedRect};
use platforms::target::{OS, TARGET_OS};
use winit::{event::VirtualKeyCode, window::Theme};

use crate::{
    math::{Point, Scale, Scaled, Size, Vector},
    shape::Shape,
    sprite::RenderedSprite,
    text::{font::Font, prepared::PreparedSpan},
};

/// An individual render instruction.
#[derive(Debug)]
pub enum Element {
    /// A rendered sprite.
    Sprite {
        /// The sprite being rendered.
        sprite: RenderedSprite,
        /// The current clipping rect.
        clip: Option<SizedRect<u32, Pixels>>,
    },
    /// A rendered span of text.
    Text {
        /// The span being rendered.
        span: PreparedSpan,
        /// The current clipping rect.
        clip: Option<SizedRect<u32, Pixels>>,
    },
    /// A rendered shape.
    Shape(Shape<Pixels>),
}

/// An event instructing how to render frames.
pub enum SceneEvent {
    /// Begin a new frame with the given size.
    BeginFrame {
        /// The frame size to render.
        size: Size<u32, Pixels>,
    },
    /// Render an element.
    Render(Element),
    /// Finish the current frame.
    EndFrame,
}

/// The main rendering destination, usually interacted with through [`Target`].
#[derive(Debug)]
pub struct Scene {
    /// The virtual key codes curently depressed.
    pub keys_pressed: HashSet<VirtualKeyCode>,
    scale_factor: DisplayScale<f32>,
    size: Size<u32, Pixels>,
    event_sender: flume::Sender<SceneEvent>,
    now: Option<Instant>,
    elapsed: Option<Duration>,
    fonts: HashMap<String, Vec<Font>>,
    system_theme: Theme,
}

impl From<Arc<Scene>> for Target {
    fn from(scene: Arc<Scene>) -> Self {
        Self {
            scene,
            clip: None,
            offset: None,
        }
    }
}

impl From<Scene> for Target {
    fn from(scene: Scene) -> Self {
        Self::from(Arc::new(scene))
    }
}

/// A render target
#[derive(Clone, Debug)]
pub struct Target {
    /// The scene to draw into.
    pub scene: Arc<Scene>,
    /// The curent clipping rect. All drawing calls will be clipped to this
    /// area.
    pub clip: Option<SizedRect<u32, Pixels>>,
    /// The current offset (translation) of drawing calls.
    pub offset: Option<Vector<f32, Pixels>>,
}

impl Target {
    /// Returns a new [`Target`] with the intersection of `new_clip` an the
    /// current `clip`, if any. The scene and offset are cloned.
    #[must_use]
    pub fn clipped_to(&self, new_clip: SizedRect<u32, Pixels>) -> Self {
        Self {
            scene: self.scene.clone(),
            clip: Some(match &self.clip {
                Some(existing_clip) => existing_clip
                    .intersection(&new_clip)
                    .unwrap_or_default()
                    .as_sized(),
                None => new_clip
                    .intersection(&SizedRect::new(Point::default(), self.size_in_pixels()))
                    .unwrap_or_default()
                    .as_sized(),
            }),
            offset: self.offset,
        }
    }

    /// Returns a new [`Target`] offset by `delta` from the current `offset`, if
    /// any. The scene and clipping rect are cloned.
    #[must_use]
    pub fn offset_by(&self, delta: Vector<f32, Pixels>) -> Self {
        Self {
            scene: self.scene.clone(),
            clip: self.clip,
            offset: Some(match self.offset {
                Some(offset) => offset + delta,
                None => delta,
            }),
        }
    }

    /// Translates `point` by the current `offset`, if any.
    #[must_use]
    pub fn offset_point(&self, point: Point<f32, Scaled>) -> Point<f32, Scaled> {
        match self.offset {
            Some(offset) => point + offset.to_scaled(self.scale()),
            None => point,
        }
    }

    /// Translates `point` by the current `offset`, if any.
    #[must_use]
    pub fn offset_point_raw(&self, point: Point<f32, Pixels>) -> Point<f32, Pixels> {
        match self.offset {
            Some(offset) => point + offset,
            None => point,
        }
    }

    /// Returns the scene as a mutable reference. Will only succeed if no other
    /// references exist. Not intended for use inside of `kludgine-app`.
    #[must_use]
    pub fn scene_mut(&mut self) -> Option<&mut Scene> {
        Arc::get_mut(&mut self.scene)
    }
}

impl std::ops::Deref for Target {
    type Target = Scene;

    fn deref(&self) -> &Self::Target {
        self.scene.as_ref()
    }
}

/// The state of keyboard modifier keys.
#[allow(clippy::struct_excessive_bools)]
pub struct Modifiers {
    /// If true, a control key is currently depressed.
    pub control: bool,
    /// If true, an alt key is currently depressed.
    pub alt: bool,
    /// If true, an "Operating System key" is currently depressed. For most
    /// keyboards, this is the Windows key or the Command/Apple key.
    pub operating_system: bool,
    /// If true, a shift key is currently depressed.
    pub shift: bool,
}

impl Modifiers {
    /// Returns true if the primary modifier of the current OS is depressed. For
    /// Mac and iOS, this returns `operating_system`. For all other OSes, this
    /// returns `control`.
    #[must_use]
    pub const fn primary_modifier(&self) -> bool {
        match TARGET_OS {
            OS::MacOS | OS::iOS => self.operating_system,
            _ => self.control,
        }
    }

    /// Returns true if the command key/Apple key is pressed. This only returns
    /// true if `operating_system` key is true and the current operating system
    /// is Mac or iOS.
    #[must_use]
    pub const fn command_key(&self) -> bool {
        match TARGET_OS {
            OS::MacOS | OS::iOS => self.operating_system,
            _ => false,
        }
    }
}

impl Scene {
    /// Returns a new Scene that emits [`SceneEvent`]s to `event_sender`.
    #[must_use]
    pub fn new(event_sender: flume::Sender<SceneEvent>, default_system_theme: Theme) -> Self {
        Self {
            event_sender,
            scale_factor: DisplayScale::one(),
            size: Size::default(),
            keys_pressed: HashSet::new(),
            now: None,
            elapsed: None,
            fonts: HashMap::new(),
            system_theme: default_system_theme,
        }
    }

    /// Returns the currently set [`Theme`].
    #[must_use]
    pub const fn system_theme(&self) -> Theme {
        self.system_theme
    }

    /// Sets the [`Theme`].
    pub fn set_system_theme(&mut self, system_theme: Theme) {
        self.system_theme = system_theme;
    }

    pub(crate) fn push_element(&self, element: Element) {
        drop(self.event_sender.send(SceneEvent::Render(element)));
    }

    /// Sets the size of the scene.
    pub fn set_size(&mut self, size: Size<u32, Pixels>) {
        self.size = size;
    }

    /// Sets the DPI scale.
    pub fn set_dpi_scale(&mut self, scale_factor: Scale<f32, Pixels, Points>) {
        self.scale_factor = DisplayScale::new(scale_factor, Scale::one());
    }

    /// Returns the current [`DisplayScale`].
    #[must_use]
    pub const fn scale(&self) -> &DisplayScale<f32> {
        &self.scale_factor
    }

    /// Returns true if any of `keys` are currently pressed.
    #[must_use]
    pub fn any_key_pressed(&self, keys: &[VirtualKeyCode]) -> bool {
        for key in keys {
            if self.keys_pressed.contains(key) {
                return true;
            }
        }
        false
    }

    /// Returns the currently depressed modifier keys.
    #[must_use]
    pub fn modifiers_pressed(&self) -> Modifiers {
        Modifiers {
            control: self.any_key_pressed(&[VirtualKeyCode::RControl, VirtualKeyCode::LControl]),
            alt: self.any_key_pressed(&[VirtualKeyCode::RAlt, VirtualKeyCode::LAlt]),
            shift: self.any_key_pressed(&[VirtualKeyCode::LShift, VirtualKeyCode::RShift]),
            operating_system: self.any_key_pressed(&[VirtualKeyCode::RWin, VirtualKeyCode::LWin]),
        }
    }

    /// Begins a new frame with the current size.
    pub fn start_frame(&mut self) {
        let last_start = self.now;
        self.now = Some(Instant::now());
        self.elapsed = match last_start {
            Some(last_start) => self.now.unwrap().checked_duration_since(last_start),
            None => None,
        };
        drop(
            self.event_sender
                .send(SceneEvent::BeginFrame { size: self.size }),
        );
    }

    /// Ends the current frame, allowing it to be rendered.
    pub fn end_frame(&self) {
        drop(self.event_sender.send(SceneEvent::EndFrame));
    }

    /// Returns the current size of the scene in [`Scaled`] units.
    #[must_use]
    pub fn size(&self) -> Size<f32, Scaled> {
        self.size.cast::<f32>().to_scaled(self.scale())
    }

    /// Returns the current size of the scene in [`Pixels`] units.
    #[must_use]
    pub const fn size_in_pixels(&self) -> Size<u32, Pixels> {
        self.size
    }

    /// Returns the [`Instant`] when the frame began.
    #[must_use]
    pub fn now(&self) -> Instant {
        self.now.expect("now() called without starting a frame")
    }

    /// Returns the elapsed [`Duration`] since the scene was created.
    #[must_use]
    pub const fn elapsed(&self) -> Option<Duration> {
        self.elapsed
    }

    /// Returns true if this is the first frame being rendered.
    #[must_use]
    pub const fn is_initial_frame(&self) -> bool {
        self.elapsed.is_none()
    }

    // pub fn register_font(&mut self, font: &Font) {
    //     let family = font.family().expect("Unable to register VecFonts");
    //     self.fonts
    //         .entry(family)
    //         .and_modify(|fonts| fonts.push(font.clone()))
    //         .or_insert_with(|| vec![font.clone()]);
    // }

    // pub fn lookup_font(
    //     &self,
    //     family: &str,
    //     weight: Weight,
    //     style: FontStyle,
    // ) -> kludgine::Result<Font> {
    //     let fonts = self.fonts.get(family);

    //     match fonts {
    //         Some(fonts) => {
    //             let mut closest_font = None;
    //             let mut closest_weight = None;

    //             for font in fonts.iter() {
    //                 let font_weight = font.weight();
    //                 let font_style = font.style();

    //                 if font_weight == weight && font_style == style {
    //                     return Ok(font.clone());
    //                 } else {
    //                     // If it's not the right style, we want to heavily
    // penalize the score                     // But if no font matches the
    // style, we should pick the weight that matches                     // best
    // in another style.                     let style_multiplier = if
    // font_style == style { 1 } else { 10 };                     let delta =
    // (font.weight().to_number() as i32 - weight.to_number() as i32)
    //                         .abs()
    //                         * style_multiplier;

    //                     if closest_weight.is_none() || closest_weight.unwrap() >
    // delta {                         closest_weight = Some(delta);
    //                         closest_font = Some(font.clone());
    //                     }
    //                 }
    //             }

    //             closest_font.ok_or_else(||
    // KludgineError::FontFamilyNotFound(family.to_owned()))         }
    //         None => Err(KludgineError::FontFamilyNotFound(family.to_owned())),
    //     }
    // }
}
