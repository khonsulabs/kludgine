use crate::{
    math::{Rect, Size},
    scene::SceneTarget,
    style::EffectiveStyle,
    ui::{global_arena, Index, Layout, LayoutSolver, SceneContext, StyledContext},
    KludgineHandle, KludgineResult,
};
use std::collections::HashMap;

pub struct SharedLayoutData {
    layout_solvers: HashMap<Index, Box<dyn LayoutSolver>>,
    pub(crate) layouts: HashMap<Index, Layout>,
    effective_styles: HashMap<Index, EffectiveStyle>,
}

impl SharedLayoutData {
    pub fn new(
        layout_solvers: HashMap<Index, Box<dyn LayoutSolver>>,
        effective_styles: HashMap<Index, EffectiveStyle>,
    ) -> Self {
        Self {
            layout_solvers,
            effective_styles,
            layouts: HashMap::new(),
        }
    }
}

pub struct LayoutContext {
    base: StyledContext,
    layout: KludgineHandle<SharedLayoutData>,
}

impl std::ops::Deref for LayoutContext {
    type Target = SceneContext;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl LayoutContext {
    pub(crate) fn new<I: Into<Index>>(
        index: I,
        scene: SceneTarget,
        effective_style: EffectiveStyle,
        layout: KludgineHandle<SharedLayoutData>,
    ) -> Self {
        Self {
            base: StyledContext::new(index, scene, effective_style),
            layout,
        }
    }

    pub async fn clone_for<I: Into<Index>>(&self, index: I) -> Self {
        let index = index.into();
        let data = self.layout.read().await;
        let effective_style = data.effective_styles.get(&index).unwrap();
        Self {
            base: StyledContext::new(index, self.scene().clone(), effective_style.clone()),
            layout: self.layout.clone(),
        }
    }

    pub fn styled_context(&mut self) -> &'_ mut StyledContext {
        &mut self.base
    }

    pub async fn layout_within(
        &mut self,
        index: impl Into<Index>,
        bounds: &Rect,
    ) -> KludgineResult<HashMap<Index, Layout>> {
        let index = index.into();
        let node = global_arena().get(index).await.unwrap();
        let content_size = node
            .content_size(
                self.styled_context(),
                &Size::new(Some(bounds.size.width), Some(bounds.size.height)),
            )
            .await?;
        let data = self.layout.read().await;
        let solver = data.layout_solvers.get(&index.into()).unwrap();

        let mut context = self.clone_for(index).await;
        solver
            .layout_within(bounds, &content_size, &mut context)
            .await
    }

    pub async fn layout_for(&self, index: impl Into<Index>) -> Option<Layout> {
        let data = self.layout.read().await;
        data.layouts.get(&index.into()).cloned()
    }

    pub async fn insert_layout(&self, index: impl Into<Index>, layout: Layout) {
        let mut data = self.layout.write().await;
        data.layouts.insert(index.into(), layout);
    }

    pub async fn layouts(&self) -> HashMap<Index, Layout> {
        let data = self.layout.read().await;
        data.layouts.clone()
    }
}
