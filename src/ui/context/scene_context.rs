use crate::{
    scene::SceneTarget,
    ui::{Context, Index},
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

impl SceneContext {
    pub(crate) fn new<I: Into<Index>>(index: I, scene: SceneTarget) -> Self {
        Self {
            base: Context::new(index),
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
}
