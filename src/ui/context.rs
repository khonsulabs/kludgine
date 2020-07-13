use crate::{
    scene::SceneTarget,
    ui::{HierarchicalArena, Index},
    KludgineHandle,
};

pub struct Context {
    index: Index,
    arena: KludgineHandle<HierarchicalArena>,
    scene: SceneTarget,
}

impl Context {
    pub fn index(&self) -> Index {
        self.index
    }

    pub(crate) fn arena(&self) -> &KludgineHandle<HierarchicalArena> {
        &self.arena
    }

    pub fn scene(&self) -> &SceneTarget {
        &self.scene
    }
}

impl Context {
    pub(crate) fn new(
        index: Index,
        arena: KludgineHandle<HierarchicalArena>,
        scene: SceneTarget,
    ) -> Self {
        Self {
            index,
            arena,
            scene,
        }
    }
}
