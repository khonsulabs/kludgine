use std::{collections::HashSet, fmt::Debug, sync::Arc};

use crate::{
    color::Color,
    math::{Raw, Scaled},
    scene::Scene,
    text::font::FontStyle,
};
use euclid::{Length, Scale};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Weight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
    Other(u16),
}

impl Default for Weight {
    fn default() -> Self {
        ttf_parser::Weight::default().into()
    }
}

impl UnscaledStyleComponent for Weight {}
impl UnscaledStyleComponent for FontStyle {}

impl Weight {
    pub fn to_number(self) -> u16 {
        let ttf: ttf_parser::Weight = self.into();
        ttf.to_number()
    }
}

impl From<ttf_parser::Weight> for Weight {
    fn from(weight: ttf_parser::Weight) -> Self {
        match weight {
            ttf_parser::Weight::Thin => Weight::Thin,
            ttf_parser::Weight::ExtraLight => Weight::ExtraLight,
            ttf_parser::Weight::Light => Weight::Light,
            ttf_parser::Weight::Normal => Weight::Normal,
            ttf_parser::Weight::Medium => Weight::Medium,
            ttf_parser::Weight::SemiBold => Weight::SemiBold,
            ttf_parser::Weight::Bold => Weight::Bold,
            ttf_parser::Weight::ExtraBold => Weight::ExtraBold,
            ttf_parser::Weight::Black => Weight::Black,
            ttf_parser::Weight::Other(value) => Weight::Other(value),
        }
    }
}

impl Into<ttf_parser::Weight> for Weight {
    fn into(self) -> ttf_parser::Weight {
        match self {
            Weight::Thin => ttf_parser::Weight::Thin,
            Weight::ExtraLight => ttf_parser::Weight::ExtraLight,
            Weight::Light => ttf_parser::Weight::Light,
            Weight::Normal => ttf_parser::Weight::Normal,
            Weight::Medium => ttf_parser::Weight::Medium,
            Weight::SemiBold => ttf_parser::Weight::SemiBold,
            Weight::Bold => ttf_parser::Weight::Bold,
            Weight::ExtraBold => ttf_parser::Weight::ExtraBold,
            Weight::Black => ttf_parser::Weight::Black,
            Weight::Other(value) => ttf_parser::Weight::Other(value),
        }
    }
}

#[derive(Debug)]
pub struct Style {
    components: Map<dyn StyleComponent>,
}

impl Clone for Style {
    fn clone(&self) -> Self {
        let mut new_map = Map::new();

        for value in self.components.as_ref().iter() {
            // SAFETY: We're always using the same type_id from the variable we're cloning
            unsafe {
                new_map
                    .as_mut()
                    .insert(value.type_id(), value.clone_to_any_style_component());
            }
        }

        Self {
            components: new_map,
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            components: Map::new(),
        }
    }
}

impl Style {
    pub fn new() -> Self {
        Self {
            components: Map::new(),
        }
    }

    pub fn with<T: StyleComponent>(mut self, component: T) -> Self {
        self.components.insert(StyleComponentWrapper(component));
        self
    }

    pub fn get<T: StyleComponent>(&self) -> Option<&T> {
        self.components
            .get::<StyleComponentWrapper<T>>()
            .map(|w| &w.0)
    }
}

pub trait CloneToAnyStyleComponent: Send + Sync {
    fn clone_to_any_style_component(&self) -> Box<dyn StyleComponent>;
}

impl<T: StyleComponent + Clone + ?Sized> CloneToAnyStyleComponent for T {
    fn clone_to_any_style_component(&self) -> Box<dyn StyleComponent> {
        Box::new(self.clone())
    }
}

impl StyleComponent for FontSize<Raw> {
    fn apply(&self, _scale: Scale<f32, Scaled, Raw>, map: &mut ComponentCollection) {
        map.push(*self);
    }
}

impl StyleComponent for FontSize<Scaled> {
    fn apply(&self, scale: Scale<f32, Scaled, Raw>, map: &mut ComponentCollection) {
        map.push(FontSize(self.0 * scale));
    }
}

pub struct ComponentCollection {
    map: Map<dyn StyleComponent>,
}

impl ComponentCollection {
    pub fn push<T: StyleComponent>(&mut self, component: T) {
        self.map.insert(StyleComponentWrapper(component));
    }
}

pub trait StyleComponent:
    anymap::any::CloneAny + CloneToAnyStyleComponent + Send + Sync + Debug + 'static
{
    fn apply(&self, scale: Scale<f32, Scaled, Raw>, map: &mut ComponentCollection);
}

pub trait UnscaledStyleComponent: Clone + Send + Sync + Debug + 'static {}

struct StyleComponentWrapper<T>(T);

impl<T: StyleComponent> IntoBox<dyn StyleComponent> for StyleComponentWrapper<T> {
    fn into_box(self) -> Box<dyn StyleComponent> {
        Box::new(self.0)
    }
}

impl<T: UnscaledStyleComponent> StyleComponent for T {
    fn apply(&self, _scale: Scale<f32, Scaled, Raw>, map: &mut ComponentCollection) {
        map.push(self.clone());
    }
}

impl UncheckedAnyExt for dyn StyleComponent {
    unsafe fn downcast_ref_unchecked<T: anymap::any::Any>(&self) -> &T {
        &*(self as *const Self as *const T)
    }
    unsafe fn downcast_mut_unchecked<T: anymap::any::Any>(&mut self) -> &mut T {
        &mut *(self as *mut Self as *mut T)
    }
    unsafe fn downcast_unchecked<T: anymap::any::Any>(self: Box<Self>) -> Box<T> {
        Box::from_raw(Box::into_raw(self) as *mut T)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}
impl UnscaledStyleComponent for Alignment {}

impl Default for Alignment {
    fn default() -> Self {
        Self::Left
    }
}

impl Style {
    pub fn inherit_from(&self, parent: &Style) -> Self {
        let mut merged_components = Map::<dyn StyleComponent>::new();
        let self_types = self
            .components
            .as_ref()
            .iter()
            .map(|a| a.type_id())
            .collect::<HashSet<_>>();
        let parent_types = parent
            .components
            .as_ref()
            .iter()
            .map(|a| a.type_id())
            .collect::<HashSet<_>>();

        for type_id in self_types.union(&parent_types) {
            let value = if self_types.contains(type_id) {
                let raw_map = self.components.as_ref();
                raw_map.get(type_id).unwrap().clone_to_any_style_component()
            } else {
                let raw_map = parent.components.as_ref();
                raw_map.get(type_id).unwrap().clone_to_any_style_component()
            };
            // SAFETY: As long as AnyMap provides valid resulting type ids from iterators,
            // the type here is guaranteed to match the type pulled from the map;
            unsafe {
                merged_components.as_mut().insert(*type_id, value);
            }
        }
        Self {
            components: merged_components,
        }
    }

    pub async fn effective_style(&self, scene: &Scene) -> EffectiveStyle {
        let mut component_collection = ComponentCollection { map: Map::new() };
        let scale = scene.scale_factor().await;

        for component in self.components.as_ref().iter() {
            component.apply(scale, &mut component_collection);
        }

        EffectiveStyle {
            components: Arc::new(component_collection.map),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct StyleSheet {
    pub normal: Style,
    pub hover: Style,
    pub focus: Style,
    pub active: Style,
}

impl From<Style> for StyleSheet {
    fn from(style: Style) -> Self {
        Self {
            normal: style.clone(),
            active: style.clone(),
            hover: style.clone(),
            focus: style,
        }
    }
}

use anymap::{any::IntoBox, any::UncheckedAnyExt, Map};

#[derive(Clone, Debug)]
pub struct EffectiveStyle {
    components: Arc<Map<dyn StyleComponent>>,
}

impl EffectiveStyle {
    pub fn get<T: StyleComponent>(&self) -> Option<&T> {
        self.components
            .get::<StyleComponentWrapper<T>>()
            .map(|w| &w.0)
    }

    pub fn get_or_default<T: StyleComponent + Default + Clone>(&self) -> T {
        self.components
            .get::<StyleComponentWrapper<T>>()
            .map(|w| w.0.clone())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct FontFamily(pub String);
impl UnscaledStyleComponent for FontFamily {}
impl Default for FontFamily {
    fn default() -> Self {
        Self("sans-serif".to_owned())
    }
}

impl<T> From<T> for FontFamily
where
    T: ToString,
{
    fn from(family: T) -> Self {
        Self(family.to_string())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct FontSize<Unit: Default + Copy>(pub Length<f32, Unit>);

impl Default for FontSize<Scaled> {
    fn default() -> Self {
        Self::new(14.)
    }
}

impl<Unit: Default + Copy> FontSize<Unit> {
    pub fn new(value: f32) -> Self {
        Self(Length::new(value))
    }

    pub fn get(&self) -> f32 {
        self.0.get()
    }

    pub fn length(&self) -> Length<f32, Unit> {
        self.0
    }
}

impl FontSize<Scaled> {
    pub fn points(points: f32) -> Self {
        Self::new(points)
    }
}

#[derive(Debug, Clone)]
pub struct BackgroundColor(pub Color);
impl UnscaledStyleComponent for BackgroundColor {}

impl Default for BackgroundColor {
    fn default() -> Self {
        BackgroundColor(Color::WHITE)
    }
}

#[derive(Debug, Clone)]
pub struct ForegroundColor(pub Color);
impl UnscaledStyleComponent for ForegroundColor {}

impl Default for ForegroundColor {
    fn default() -> Self {
        ForegroundColor(Color::BLACK)
    }
}
