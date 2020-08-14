use crate::{
    style::StyleSheet,
    ui::{HierarchicalArena, Index, Layout, UIState},
};
mod layout_context;
mod scene_context;
mod styled_context;
pub use self::{
    layout_context::{LayoutContext, LayoutEngine},
    scene_context::SceneContext,
    styled_context::StyledContext,
};

pub struct Context {
    index: Index,
    arena: HierarchicalArena,
    ui_state: UIState,
}

impl Context {
    pub fn index(&self) -> Index {
        self.index
    }
}

impl Context {
    pub(crate) fn new<I: Into<Index>>(
        index: I,
        arena: HierarchicalArena,
        ui_state: UIState,
    ) -> Self {
        Self {
            index: index.into(),
            arena,
            ui_state,
        }
    }

    pub async fn set_parent<I: Into<Index>>(&self, parent: Option<I>) {
        self.arena
            .set_parent(self.index, parent.map(|p| p.into()))
            .await
    }

    pub async fn add_child<I: Into<Index>>(&self, child: I) {
        let child = child.into();

        self.arena.set_parent(child, Some(self.index)).await
    }

    pub async fn remove<I: Into<Index>>(&self, element: I) {
        self.arena.remove(element).await;
    }

    pub async fn children(&self) -> Vec<Index> {
        self.arena.children(&Some(self.index)).await
    }

    pub fn clone_for<I: Into<Index>>(&self, index: I) -> Self {
        Self {
            index: index.into(),
            arena: self.arena.clone(),
            ui_state: self.ui_state.clone(),
        }
    }

    pub async fn last_layout(&self) -> Layout {
        let node = self.arena.get(self.index).await.unwrap();
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
        let node = self.arena.get(self.index).await.unwrap();
        node.style_sheet().await
    }

    pub async fn set_style_sheet(&self, sheet: StyleSheet) {
        let node = self.arena.get(self.index).await.unwrap();
        node.set_style_sheet(sheet).await
    }
}
