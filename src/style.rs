use std::{any::TypeId, collections::HashMap, collections::HashSet, fmt::Debug};

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

impl UnscaledStyleComponent<Raw> for Weight {}
impl UnscaledStyleComponent<Scaled> for Weight {}
impl UnscaledStyleComponent<Raw> for FontStyle {}
impl UnscaledStyleComponent<Scaled> for FontStyle {}

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
pub struct Style<Unit: 'static> {
    components: HashMap<TypeId, Box<dyn AnyStyleComponent<Unit>>>,
}

impl<Unit: Send + Sync + Debug + Clone> Clone for Style<Unit> {
    fn clone(&self) -> Self {
        let mut new_map = HashMap::<TypeId, Box<dyn AnyStyleComponent<Unit>>>::new();

        for (type_id, value) in self.components.iter() {
            new_map.insert(*type_id, value.clone_to_style_component());
        }

        Self {
            components: new_map,
        }
    }
}

impl<Unit> Default for Style<Unit> {
    fn default() -> Self {
        Self {
            components: HashMap::new(),
        }
    }
}

pub trait AnyStyleComponent<Unit>: StyleComponent<Unit> + Send + Sync + Debug + 'static {
    fn as_any(&self) -> &'_ dyn std::any::Any;
    fn clone_to_style_component(&self) -> Box<dyn AnyStyleComponent<Unit>>;
}

// impl<Unit: Send + Sync + Debug + 'static> AnyStyleComponent<Unit> for StyleComponentWrapper<Unit> {
//     fn as_any(&self) -> &'_ dyn std::any::Any {
//         self
//     }
// }

impl<T: StyleComponent<Unit> + Clone, Unit: Send + Sync + Debug + 'static> AnyStyleComponent<Unit>
    for T
{
    fn as_any(&self) -> &'_ dyn std::any::Any {
        self
    }

    fn clone_to_style_component(&self) -> Box<dyn AnyStyleComponent<Unit>> {
        Box::new(self.clone())
    }
}

// impl<Unit: Send + Sync + Debug + 'static> StyleComponent<Unit> for StyleComponentWrapper<Unit> {
//     fn apply(&self, scale: Scale<f32, Unit, Raw>, destination: &mut Style<Raw>) {
//         self.0.apply(scale, destination);
//     }
// }

// impl<Unit> CloneToAnyStyleComponent<Unit> for StyleComponentWrapper<Unit> {
//     fn clone_to_any_style_component(&self) -> Box<dyn StyleComponent<Unit>> {
//         Box::new(self.0.clone())
//     }
// }

impl<Unit: Send + Sync + Debug + 'static> Style<Unit> {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    pub fn push<T: StyleComponent<Unit> + Clone>(&mut self, component: T) {
        self.components
            .insert(component.type_id(), Box::new(component));
    }

    pub fn with<T: StyleComponent<Unit> + Clone>(mut self, component: T) -> Self {
        self.push(component);
        self
    }

    pub fn get<T: StyleComponent<Unit>>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();

        self.components
            .get(&type_id)
            .map(|w| {
                let component_as_any = w.as_any();
                component_as_any.downcast_ref::<T>()
            })
            .flatten()
    }

    pub fn get_or_default<T: StyleComponent<Unit> + Default + Clone>(&self) -> T {
        self.get::<T>().cloned().unwrap_or_default()
    }
}

impl StyleComponent<Scaled> for FontSize<Scaled> {
    fn apply(&self, scale: Scale<f32, Scaled, Raw>, map: &mut Style<Raw>) {
        map.push(FontSize(self.0 * scale));
    }
}

impl StyleComponent<Raw> for FontSize<Raw> {
    fn apply(&self, _scale: Scale<f32, Raw, Raw>, map: &mut Style<Raw>) {
        map.push(FontSize(self.0));
    }
}

pub struct ComponentCollection<Unit: 'static> {
    map: HashMap<TypeId, Box<dyn StyleComponent<Unit>>>,
}

impl<Unit> ComponentCollection<Unit> {
    pub fn push<T: StyleComponent<Unit>>(&mut self, component: T) {
        self.map.insert(component.type_id(), Box::new(component));
    }
}

pub trait StyleComponent<Unit>: std::any::Any + Send + Sync + Debug + 'static {
    fn apply(&self, scale: Scale<f32, Unit, Raw>, destination: &mut Style<Raw>);
}

pub trait UnscaledStyleComponent<Unit>:
    AnyStyleComponent<Unit> + Clone + Send + Sync + Debug + 'static
{
}

impl<T, Unit> StyleComponent<Unit> for T
where
    T: UnscaledStyleComponent<Unit> + UnscaledStyleComponent<Raw>,
    Unit: Clone + Send + Sync + Debug + 'static,
{
    fn apply(&self, _scale: Scale<f32, Unit, Raw>, destination: &mut Style<Raw>) {
        destination.push(self.clone());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}
impl UnscaledStyleComponent<Raw> for Alignment {}
impl UnscaledStyleComponent<Scaled> for Alignment {}

impl Default for Alignment {
    fn default() -> Self {
        Self::Left
    }
}

impl<Unit: Send + Sync + Debug + 'static> Style<Unit> {
    pub fn inherit_from(&self, parent: &Style<Unit>) -> Self {
        let mut merged_components = HashMap::<TypeId, Box<dyn AnyStyleComponent<Unit>>>::new();
        let self_types = self.components.keys().cloned().collect::<HashSet<_>>();
        let parent_types = parent.components.keys().cloned().collect::<HashSet<_>>();

        for type_id in self_types.union(&parent_types) {
            let value = if self_types.contains(type_id) {
                self.components
                    .get(type_id)
                    .unwrap()
                    .clone_to_style_component()
            } else {
                parent
                    .components
                    .get(type_id)
                    .unwrap()
                    .clone_to_style_component()
            };
            merged_components.insert(*type_id, value);
        }
        Self {
            components: merged_components,
        }
    }
}

impl Style<Scaled> {
    pub async fn effective_style(&self, scene: &Scene) -> Style<Raw> {
        let mut style = Style::new();
        let scale = scene.scale_factor().await;

        for component in self.components.values() {
            component.apply(scale, &mut style);
        }

        style
    }
}

#[derive(Default, Clone, Debug)]
pub struct StyleSheet {
    pub normal: Style<Scaled>,
    pub hover: Style<Scaled>,
    pub focus: Style<Scaled>,
    pub active: Style<Scaled>,
}

impl From<Style<Scaled>> for StyleSheet {
    fn from(style: Style<Scaled>) -> Self {
        Self {
            normal: style.clone(),
            active: style.clone(),
            hover: style.clone(),
            focus: style,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FontFamily(pub String);
impl UnscaledStyleComponent<Raw> for FontFamily {}
impl UnscaledStyleComponent<Scaled> for FontFamily {}
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

#[derive(Debug, Clone)]
pub struct BackgroundColor(pub Color);
impl UnscaledStyleComponent<Raw> for BackgroundColor {}
impl UnscaledStyleComponent<Scaled> for BackgroundColor {}

impl Default for BackgroundColor {
    fn default() -> Self {
        BackgroundColor(Color::WHITE)
    }
}

#[derive(Debug, Clone)]
pub struct ForegroundColor(pub Color);
impl UnscaledStyleComponent<Raw> for ForegroundColor {}
impl UnscaledStyleComponent<Scaled> for ForegroundColor {}

impl Default for ForegroundColor {
    fn default() -> Self {
        ForegroundColor(Color::BLACK)
    }
}
