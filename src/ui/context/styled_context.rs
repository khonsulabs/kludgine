use crate::{
    scene::SceneTarget,
    style::EffectiveStyle,
    ui::{Index, SceneContext},
};

pub struct StyledContext {
    base: SceneContext,
    effective_style: EffectiveStyle,
}

impl std::ops::Deref for StyledContext {
    type Target = SceneContext;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl StyledContext {
    pub(crate) fn new<I: Into<Index>>(
        index: I,
        scene: SceneTarget,
        effective_style: EffectiveStyle,
    ) -> Self {
        Self {
            base: SceneContext::new(index, scene),
            effective_style,
        }
    }

    pub fn clone_for<I: Into<Index>>(&self, index: I) -> Self {
        Self {
            base: self.base.clone_for(index),
            effective_style: self.effective_style.clone(),
        }
    }

    pub fn from_scene_context(effective_style: EffectiveStyle, base: SceneContext) -> Self {
        Self {
            base,
            effective_style,
        }
    }

    pub fn effective_style(&self) -> &'_ EffectiveStyle {
        &self.effective_style
    }
}
