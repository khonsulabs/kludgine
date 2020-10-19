use crate::{
    math::{Raw, Scaled},
    scene::Scene,
};
use euclid::Scale;
use std::{any::TypeId, collections::HashMap, collections::HashSet, fmt::Debug};

mod alignment;
mod any;
mod background_color;
mod font_family;
mod font_size;
mod font_style;
mod foreground_color;
mod weight;
pub use self::{
    alignment::Alignment, any::AnyStyleComponent, background_color::BackgroundColor,
    font_family::FontFamily, font_size::FontSize, font_style::FontStyle,
    foreground_color::ForegroundColor, weight::Weight,
};

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
