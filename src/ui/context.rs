use async_handle::Handle;

use crate::{
    scene::Target,
    style::StyleSheet,
    ui::{
        node::ThreadsafeAnyMap, Entity, EntityBuilder, HierarchicalArena, Index, Indexable,
        InteractiveComponent, Layout, UIState,
    },
};
mod layout_context;
mod styled_context;
pub use self::{
    layout_context::{LayoutContext, LayoutEngine},
    styled_context::StyledContext,
};
use super::{LayerIndex, LayerIndexable};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct Context {
    layer_index: LayerIndex,
    arena: HierarchicalArena,
    ui_state: UIState,
    scene: Target,
}

impl Context {
    pub(crate) fn new<I: LayerIndexable>(
        index: I,
        arena: HierarchicalArena,
        ui_state: UIState,
        scene: Target,
    ) -> Self {
        Self {
            layer_index: index.layer_index(),
            arena,
            ui_state,
            scene,
        }
    }

    pub fn insert_new_entity<T: InteractiveComponent + 'static, I: Indexable, Message>(
        &self,
        parent: I,
        component: T,
    ) -> EntityBuilder<T, Message> {
        let component = Handle::new(component);
        let mut components = ThreadsafeAnyMap::new();
        components.insert(component);
        EntityBuilder {
            components,
            scene: self.scene().clone(),
            parent: Some(parent.index()),
            interactive: true,
            layer: self.layer_index.layer.clone(),
            ui_state: self.ui_state().clone(),
            arena: self.arena().clone(),
            style_sheet: Default::default(),
            callback: None,
            _marker: Default::default(),
        }
    }

    pub fn index(&self) -> Index {
        self.layer_index.index
    }

    pub fn layer_index(&self) -> LayerIndex {
        self.layer_index.clone()
    }

    pub fn scene(&self) -> &'_ Target {
        &self.scene
    }

    pub fn scene_mut(&mut self) -> &'_ mut Target {
        &mut self.scene
    }

    pub fn entity<T: InteractiveComponent>(&self) -> Entity<T> {
        Entity::new(self.clone())
    }

    pub async fn set_parent<I: Indexable>(&self, parent: Option<I>) {
        self.arena.set_parent(self.layer_index.index, parent).await
    }

    pub async fn add_child<I: Indexable>(&self, child: I) {
        self.arena
            .set_parent(child, Some(self.layer_index.index))
            .await
    }

    pub async fn remove<I: Indexable>(&self, element: &I) {
        self.arena.remove(element).await;
    }

    pub async fn children(&self) -> Vec<Index> {
        self.arena.children(&Some(self.layer_index.index)).await
    }

    pub fn clone_for<I: Indexable>(&self, index: &I) -> Self {
        Self {
            layer_index: LayerIndex {
                index: index.index(),
                layer: self.layer_index.layer.clone(),
            },
            arena: self.arena.clone(),
            ui_state: self.ui_state.clone(),
            scene: self.scene.clone(),
        }
    }

    pub async fn last_layout(&self) -> Layout {
        let node = self.arena.get(&self.layer_index.index).await.unwrap();
        node.last_layout().await
    }

    pub(crate) fn arena(&self) -> &'_ HierarchicalArena {
        &self.arena
    }

    pub(crate) fn ui_state(&self) -> &'_ UIState {
        &self.ui_state
    }

    pub async fn activate(&self) {
        self.layer_index
            .layer
            .activate(self.layer_index.index, &self.ui_state)
            .await;
    }

    pub async fn deactivate(&self) {
        self.layer_index.layer.deactivate(&self.ui_state).await;
    }

    pub async fn style_sheet(&self) -> StyleSheet {
        let node = self.arena.get(&self.layer_index.index).await.unwrap();
        node.style_sheet().await
    }

    pub async fn focus(&self) {
        self.layer_index
            .layer
            .focus_on(Some(self.layer_index.index), &self.ui_state)
            .await;
    }

    pub async fn is_focused(&self) -> bool {
        self.layer_index
            .layer
            .focus()
            .await
            .map(|focus| focus == self.layer_index.index)
            .unwrap_or_default()
    }

    pub async fn blur(&self) {
        self.layer_index.layer.focus_on(None, &self.ui_state).await;
    }

    pub async fn set_style_sheet(&self, sheet: StyleSheet) {
        let node = self.arena.get(&self.layer_index.index).await.unwrap();
        node.set_style_sheet(sheet).await
    }

    pub async fn set_needs_redraw(&self) {
        self.ui_state.set_needs_redraw().await;
    }

    pub async fn estimate_next_frame(&self, duration: Duration) {
        self.ui_state.estimate_next_frame(duration).await;
    }

    pub async fn estimate_next_frame_instant(&self, instant: Instant) {
        self.ui_state.estimate_next_frame_instant(instant).await;
    }
}
