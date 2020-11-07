use crate::{
    math::{Raw, Scaled, Size, Surround},
    scene::Target,
    style::Style,
    ui::{Context, HierarchicalArena, Index, Indexable, UIState},
    KludgineError, KludgineResult,
};
use std::{collections::HashMap, sync::Arc};

pub struct StyledContext {
    base: Context,
    effective_styles: Arc<HashMap<Index, Style<Raw>>>,
}

impl std::ops::Deref for StyledContext {
    type Target = Context;

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
        scene: Target,
        effective_styles: Arc<HashMap<Index, Style<Raw>>>,
        arena: HierarchicalArena,
        ui_state: UIState,
    ) -> Self {
        Self {
            base: Context::new(index, arena, ui_state, scene),
            effective_styles,
        }
    }

    pub fn clone_for<I: Indexable>(&self, index: &I) -> Self {
        Self {
            base: self.base.clone_for(index),
            effective_styles: self.effective_styles.clone(),
        }
    }

    pub fn from_scene_context(
        effective_styles: Arc<HashMap<Index, Style<Raw>>>,
        base: Context,
    ) -> Self {
        Self {
            base,
            effective_styles,
        }
    }

    pub fn effective_style(&self) -> &'_ Style<Raw> {
        &self.effective_styles.get(&self.index()).unwrap()
    }

    pub async fn content_size(
        &self,
        index: &impl Indexable,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let node = self
            .arena
            .get(index)
            .await
            .ok_or(KludgineError::InvalidIndex)?;

        let mut context = self.clone_for(index);
        node.content_size(&mut context, constraints).await
    }

    pub async fn content_size_with_padding(
        &self,
        index: &impl Indexable,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<(Size<f32, Scaled>, Surround<f32, Scaled>)> {
        let node = self
            .arena
            .get(index)
            .await
            .ok_or(KludgineError::InvalidIndex)?;

        let mut context = self.clone_for(index);
        node.content_size_with_padding(&mut context, constraints)
            .await
    }
}
