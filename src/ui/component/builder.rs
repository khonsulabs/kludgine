use crate::{
    math::Scaled,
    scene::Target,
    style::{
        theme::{Classes, Id, Selector},
        Style, StyleSheet,
    },
    ui::{
        node::ThreadsafeAnyMap, AbsoluteBounds, Callback, Context, Entity, HierarchicalArena,
        Indexable, InteractiveComponent, LayerIndex, Node, UILayer, UIState,
    },
    KludgineError, KludgineResult,
};
use async_handle::Handle;
use generational_arena::Index;

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
    layer: Option<UILayer>,
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
        layer: Option<&UILayer>,
        ui_state: &UIState,
        arena: &HierarchicalArena,
    ) -> EntityBuilder<C, P> {
        let mut components = ThreadsafeAnyMap::new();
        if let Some(base_classes) = component.classes() {
            components.insert(Handle::new(Classes(base_classes)));
        }

        let component = Handle::new(component);
        components.insert(component);
        EntityBuilder {
            components,
            scene: scene.clone(),
            parent,
            interactive: true,
            layer: layer.cloned(),
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

    pub fn callback<
        F: Fn(C::Event) -> Target::Message + Send + Sync + 'static,
        Target: InteractiveComponent + 'static,
    >(
        self,
        target: &Entity<Target>,
        callback: F,
    ) -> EntityBuilder<C, Target::Message> {
        let target = Context::new(
            target.index(),
            self.arena.clone(),
            self.ui_state.clone(),
            self.scene.clone(),
        );
        EntityBuilder {
            components: self.components,
            arena: self.arena,
            callback: Some(Callback::new(target, callback)),
            interactive: self.interactive,
            layer: self.layer,
            parent: self.parent,
            scene: self.scene,
            style_sheet: self.style_sheet,
            ui_state: self.ui_state,
            _marker: Default::default(),
        }
    }

    pub fn with<T: Send + Sync + 'static>(mut self, component: T) -> Self {
        self.components.insert(Handle::new(component));
        self
    }

    pub async fn with_class<S: Into<Selector>>(mut self, class: S) -> Self {
        let class = class.into();
        if let Some(classes) = self.components.get::<Handle<Classes>>() {
            let mut classes = classes.write().await;
            classes.0.push(class);
        } else {
            self.components.insert(Handle::new(Classes::from(class)));
        }
        self
    }

    pub async fn insert(mut self) -> KludgineResult<Entity<C>> {
        let theme = self.scene.theme().await;
        let id = if let Some(id) = self.components.get::<Handle<Id>>() {
            let id = id.read().await;
            Some(id.clone())
        } else {
            None
        };
        let classes = if let Some(classes) = self.components.get::<Handle<Classes>>() {
            let classes = classes.read().await;
            Some(classes.clone())
        } else {
            None
        };

        let theme_style = theme.stylesheet_for(id.as_ref(), classes.as_ref());
        self.components.insert(Handle::new(
            self.style_sheet.merge_with(&theme_style, false),
        ));

        let node = Node::from_components::<C>(self.components, self.interactive, self.callback);
        let index = self.arena.insert(self.parent, node).await;
        let layer_index = if let Some(layer) = self.layer {
            let layer_index = LayerIndex { index, layer };

            let mut context = Context::new(
                layer_index.clone(),
                self.arena.clone(),
                self.ui_state.clone(),
                self.scene.clone(),
            );
            self.arena
                .get(&layer_index.index)
                .await
                .ok_or(KludgineError::ComponentRemovedFromHierarchy)?
                .initialize(&mut context)
                .await?;

            layer_index
        } else {
            self.ui_state
                .push_layer_from_index(index, &self.arena, &self.scene)
                .await?;
            LayerIndex {
                index,
                layer: self.ui_state.top_layer().await,
            }
        };

        Ok(Entity::new(Context::new(
            layer_index,
            self.arena.clone(),
            self.ui_state,
            self.scene.clone(),
        )))
    }
}
