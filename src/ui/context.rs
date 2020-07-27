use crate::ui::{HierarchicalArena, Index, Layout};
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
}

impl Context {
    pub fn index(&self) -> Index {
        self.index
    }
}

impl Context {
    pub(crate) fn new<I: Into<Index>>(index: I, arena: HierarchicalArena) -> Self {
        Self {
            index: index.into(),
            arena,
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

    pub fn clone_for<I: Into<Index>>(&self, index: I) -> Self {
        Self {
            index: index.into(),
            arena: self.arena.clone(),
        }
    }

    pub async fn last_layout(&self) -> Layout {
        let node = self.arena.get(self.index).await.unwrap();
        node.last_layout().await
    }

    pub(crate) fn arena(&self) -> &'_ HierarchicalArena {
        &self.arena
    }
}
