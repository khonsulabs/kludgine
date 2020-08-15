use crate::{
    scene::SceneTarget,
    ui::{Context, HierarchicalArena, Index, UIState},
};

pub struct SceneContext {
    base: Context,
    scene: SceneTarget,
}

impl std::ops::Deref for SceneContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for SceneContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl SceneContext {
    pub(crate) fn new<I: Into<Index>>(
        index: I,
        scene: SceneTarget,
        arena: HierarchicalArena,
        state: UIState,
    ) -> Self {
        Self {
            base: Context::new(index, arena, state),
            scene,
        }
    }

    pub fn clone_for<I: Into<Index>>(&self, index: I) -> Self {
        Self {
            base: self.base.clone_for(index),
            scene: self.scene.clone(),
        }
    }

    pub fn scene(&self) -> &'_ SceneTarget {
        &self.scene
    }

    pub fn scene_mut(&mut self) -> &'_ mut SceneTarget {
        &mut self.scene
    }
}
