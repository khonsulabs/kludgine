use async_handle::Handle;
use generational_arena::Index;

use crate::{
    math::Scaled,
    scene::Target,
    style::{
        theme::{Classes, Id, Selector},
        Style, StyleSheet,
    },
    ui::{
        node::ThreadsafeAnyMap, AbsoluteBounds, Callback, Context, Entity, HierarchicalArena,
        InteractiveComponent, LayerIndex, Node, UILayer, UIState,
    },
    KludgineResult,
};

pub struct EntityBuilder<C, P>
where
    C: InteractiveComponent + 'static,
{
    components: ThreadsafeAnyMap,
    scene: Target,
    parent: Option<Index>,
    style_sheet: StyleSheet,
    interactive: bool,
    callback: Option<Callback<C::Event>>,
    layer: UILayer,
    ui_state: UIState,
    arena: HierarchicalArena,
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> EntityBuilder<C, P>
where
    C: InteractiveComponent + 'static,
    P: Send + Sync + 'static,
{
    pub(crate) fn new(
        parent: Option<Index>,
        component: C,
        scene: &Target,
        layer: &UILayer,
        ui_state: &UIState,
        arena: &HierarchicalArena,
    ) -> Self {
        let mut components = ThreadsafeAnyMap::new();
        if let Some(base_classes) = component.classes() {
            components.insert(Classes(base_classes));
        }

        let component = Handle::new(component);
        components.insert(component);
        Self {
            components,
            scene: scene.clone(),
            parent,
            interactive: true,
            layer: layer.clone(),
            ui_state: ui_state.clone(),
            arena: arena.clone(),
            style_sheet: Default::default(),
            callback: None,
            _marker: Default::default(),
        }
    }

    pub fn style_sheet<S: Into<StyleSheet>>(mut self, sheet: S) -> Self {
        self.style_sheet = sheet.into();
        self
    }

    pub fn normal_style(mut self, style: Style<Scaled>) -> Self {
        self.style_sheet.normal = style;
        self
    }

    pub fn hover(mut self, style: Style<Scaled>) -> Self {
        self.style_sheet.hover = style;
        self
    }

    pub fn active(mut self, style: Style<Scaled>) -> Self {
        self.style_sheet.active = style;
        self
    }

    pub fn focus(mut self, style: Style<Scaled>) -> Self {
        self.style_sheet.focus = style;
        self
    }

    pub fn bounds(mut self, bounds: AbsoluteBounds) -> Self {
        self.components.insert(Handle::new(bounds));
        self
    }

    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    pub fn callback<F: Fn(C::Event) -> P + Send + Sync + 'static>(mut self, callback: F) -> Self {
        let target = Context::new(
            LayerIndex {
                index: self.parent.unwrap(),
                layer: self.layer.clone(),
            },
            self.arena.clone(),
            self.ui_state.clone(),
            self.scene.clone(),
        );
        self.callback = Some(Callback::new(target, callback));
        self
    }

    pub fn with<T: Send + Sync + 'static>(mut self, component: T) -> Self {
        self.components.insert(component);
        self
    }

    pub fn with_class<S: Into<Selector>>(mut self, class: S) -> Self {
        let class = class.into();
        if let Some(classes) = self.components.get_mut::<Classes>() {
            classes.0.push(class);
        } else {
            self.components.insert(Classes::from(class));
        }
        self
    }

    pub async fn insert(mut self) -> KludgineResult<Entity<C>> {
        let theme = self.scene.theme().await;
        let theme_style = theme.stylesheet_for(
            self.components.get::<Id>(),
            self.components.get::<Classes>(),
        );
        self.components.insert(Handle::new(
            self.style_sheet.merge_with(&theme_style, false),
        ));
        let layer_index = {
            let node = Node::from_components::<C>(self.components, self.interactive, self.callback);
            let index = self.arena.insert(self.parent, node).await;
            let layer_index = LayerIndex {
                index,
                layer: self.layer,
            };

            let mut context = Context::new(
                layer_index.clone(),
                self.arena.clone(),
                self.ui_state.clone(),
                self.scene.clone(),
            );
            self.arena
                .get(&layer_index.index)
                .await
                .unwrap()
                .initialize(&mut context)
                .await?;

            layer_index
        };
        Ok(Entity::new(Context::new(
            layer_index,
            self.arena.clone(),
            self.ui_state,
            self.scene.clone(),
        )))
    }
}
