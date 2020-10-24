use crate::{
    prelude::Scene,
    style::StyleSheet,
    ui::{Entity, HierarchicalArena, Index, Indexable, InteractiveComponent, Layout, UIState},
};
mod layout_context;
mod styled_context;
pub use self::{
    layout_context::{LayoutContext, LayoutEngine},
    styled_context::StyledContext,
};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct Context {
    index: Index,
    arena: HierarchicalArena,
    ui_state: UIState,
    scene: Scene,
}

impl Context {
    pub(crate) fn new<I: Indexable>(
        index: I,
        arena: HierarchicalArena,
        ui_state: UIState,
        scene: Scene,
    ) -> Self {
        Self {
            index: index.index(),
            arena,
            ui_state,
            scene,
        }
    }

    pub fn index(&self) -> Index {
        self.index
    }

    pub fn scene(&self) -> &'_ Scene {
        &self.scene
    }

    pub fn scene_mut(&mut self) -> &'_ mut Scene {
        &mut self.scene
    }

    pub fn entity<T: InteractiveComponent>(&self) -> Entity<T> {
        Entity::new(self.clone())
    }

    pub async fn set_parent<I: Indexable>(&self, parent: Option<I>) {
        self.arena.set_parent(self.index, parent).await
    }

    pub async fn add_child<I: Indexable>(&self, child: I) {
        self.arena.set_parent(child, Some(self.index)).await
    }

    pub async fn remove<I: Indexable>(&self, element: &I) {
        self.arena.remove(element).await;
    }

    pub async fn children(&self) -> Vec<Index> {
        self.arena.children(&Some(self.index)).await
    }

    pub fn clone_for<I: Indexable>(&self, index: &I) -> Self {
        Self {
            index: index.index(),
            arena: self.arena.clone(),
            ui_state: self.ui_state.clone(),
            scene: self.scene.clone(),
        }
    }

    pub async fn last_layout(&self) -> Layout {
        let node = self.arena.get(&self.index).await.unwrap();
        node.last_layout().await
    }

    pub(crate) fn arena(&self) -> &'_ HierarchicalArena {
        &self.arena
    }

    pub(crate) fn ui_state(&self) -> &'_ UIState {
        &self.ui_state
    }

    pub async fn activate(&self) {
        self.ui_state.activate(self.index).await
    }

    pub async fn deactivate(&self) {
        self.ui_state.deactivate().await
    }

    pub async fn style_sheet(&self) -> StyleSheet {
        let node = self.arena.get(&self.index).await.unwrap();
        node.style_sheet().await
    }

    pub async fn focus(&self) {
        self.ui_state.focus(self.index).await
    }

    pub async fn is_focused(&self) -> bool {
        self.ui_state
            .focused()
            .await
            .map(|focus| focus == self.index)
            .unwrap_or_default()
    }

    pub async fn blur(&self) {
        self.ui_state.blur().await
    }

    pub async fn set_style_sheet(&self, sheet: StyleSheet) {
        let node = self.arena.get(&self.index).await.unwrap();
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
