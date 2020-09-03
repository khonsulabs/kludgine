use crate::{
    math::{Points, Size},
    scene::SceneTarget,
    style::EffectiveStyle,
    ui::{HierarchicalArena, Indexable, SceneContext, UIState},
    KludgineError, KludgineResult,
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

impl std::ops::DerefMut for StyledContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl StyledContext {
    pub(crate) fn new<I: Indexable>(
        index: I,
        scene: SceneTarget,
        effective_style: EffectiveStyle,
        arena: HierarchicalArena,
        ui_state: UIState,
    ) -> Self {
        Self {
            base: SceneContext::new(index, scene, arena, ui_state),
            effective_style,
        }
    }

    pub fn clone_for<I: Indexable>(&self, index: &I) -> Self {
        Self {
            base: self.base.clone_for(index),
            effective_style: self.effective_style.clone(), // TODO this isn't right
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

    pub async fn content_size(
        &self,
        index: &impl Indexable,
        constraints: &Size<Option<Points>>,
    ) -> KludgineResult<Size<Points>> {
        let node = self
            .arena
            .get(index)
            .await
            .ok_or(KludgineError::InvalidIndex)?;

        let mut context = self.clone_for(index);
        node.content_size(&mut context, constraints).await
    }
}
