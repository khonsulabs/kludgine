use crate::{
    math::{Rect, Size},
    scene::SceneTarget,
    style::EffectiveStyle,
    ui::{
        global_arena, HierarchicalArena, Index, Layout, LayoutSolver, SceneContext, StyledContext,
    },
    KludgineHandle, KludgineResult,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct LayoutEngine {
    data: KludgineHandle<LayoutEngineData>,
}

#[derive(Debug)]
struct LayoutEngineData {
    layout_solvers: HashMap<Index, KludgineHandle<Box<dyn LayoutSolver>>>,
    pub(crate) layouts: HashMap<Index, Layout>,
    indicies_to_process: VecDeque<Index>,
    render_queue: VecDeque<Index>,
    effective_styles: Arc<HashMap<Index, EffectiveStyle>>,
}

impl LayoutEngine {
    pub fn new(
        layout_solvers: HashMap<Index, KludgineHandle<Box<dyn LayoutSolver>>>,
        effective_styles: Arc<HashMap<Index, EffectiveStyle>>,
        root: impl Into<Index>,
    ) -> Self {
        let mut indicies_to_process = VecDeque::default();
        indicies_to_process.push_back(root.into());
        Self {
            data: KludgineHandle::new(LayoutEngineData {
                layout_solvers,
                effective_styles,
                indicies_to_process,
                render_queue: Default::default(),
                layouts: Default::default(),
            }),
        }
    }

    pub async fn insert_layout(&self, index: Index, layout: Layout, add_to_process_queue: bool) {
        let mut data = self.data.write().await;
        data.layouts.insert(index, layout);
        data.render_queue.push_back(index);
        if add_to_process_queue {
            data.indicies_to_process.push_back(index);
        }
    }

    pub async fn get_layout(&self, index: &Index) -> Option<Layout> {
        let data = self.data.read().await;
        data.layouts.get(index).cloned()
    }

    pub async fn effective_style(&self, index: &Index) -> Option<EffectiveStyle> {
        let data = self.data.read().await;
        data.effective_styles.get(index).cloned()
    }

    pub(crate) async fn next_to_layout(&self) -> Option<Index> {
        let mut data = self.data.write().await;
        data.indicies_to_process.pop_front()
    }

    pub(crate) async fn next_to_render(&self) -> Option<Index> {
        let mut data = self.data.write().await;
        data.render_queue.pop_front()
    }

    pub async fn solve_layout_for(
        &self,
        index: &Index,
        context: &mut LayoutContext,
        bounds: &Rect,
        content_size: &Size,
    ) -> KludgineResult<()> {
        let solver_handle = {
            let data = self.data.read().await;
            data.layout_solvers.get(index).unwrap().clone()
        };
        let solver = solver_handle.read().await;
        solver.layout_within(bounds, content_size, context).await
    }
}

pub struct LayoutContext {
    base: StyledContext,
    layout: LayoutEngine,
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
        layout: LayoutEngine,
        arena: HierarchicalArena,
    ) -> Self {
        Self {
            base: StyledContext::new(index, scene, effective_style, arena),
            layout,
        }
    }

    pub async fn clone_for<I: Into<Index>>(&self, index: I) -> Self {
        let index = index.into();
        let effective_style = self.layout.effective_style(&index).await.unwrap();
        Self {
            base: StyledContext::new(
                index,
                self.scene().clone(),
                effective_style,
                global_arena().clone(),
            ),
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
    ) -> KludgineResult<()> {
        let index = index.into();
        let node = self.arena().get(index).await.unwrap();
        let content_size = node
            .content_size(
                self.styled_context(),
                &Size::new(Some(bounds.size.width), Some(bounds.size.height)),
            )
            .await?;

        let mut solving_context = self.clone_for(index).await;
        self.layout
            .solve_layout_for(&index, &mut solving_context, bounds, &content_size)
            .await?;

        Ok(())
    }

    pub async fn layout_for(&self, index: impl Into<Index>) -> Option<Layout> {
        self.layout.get_layout(&index.into()).await
    }

    pub async fn insert_layout(&mut self, index: impl Into<Index>, layout: Layout) {
        let index = index.into();
        self.layout.insert_layout(index, layout, true).await;
    }
}
