use crate::{
    math::{Point, Rect, Size, Surround},
    scene::SceneTarget,
    style::EffectiveStyle,
    ui::{HierarchicalArena, Index, Layout, LayoutSolver, SceneContext, StyledContext, UIState},
    KludgineHandle, KludgineResult,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
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

    pub(crate) async fn layout(
        arena: &HierarchicalArena,
        ui_state: &UIState,
        root: Index,
        scene: &SceneTarget,
        hovered_indicies: HashSet<Index>,
    ) -> KludgineResult<Self> {
        let mut effective_styles = HashMap::new();
        let mut computed_styles = HashMap::new();
        let mut traverser = arena.traverse(root).await;
        let mut found_nodes = VecDeque::new();
        while let Some(index) = traverser.next().await {
            let node = arena.get(index).await.unwrap();
            let style_sheet = node.style_sheet().await;
            let mut node_style = style_sheet.normal;

            if hovered_indicies.contains(&index) {
                node_style = style_sheet.hover.inherit_from(&node_style);
            }

            if ui_state.focused().await == Some(index) {
                node_style = style_sheet.focus.inherit_from(&node_style);
            }

            if ui_state.active().await == Some(index) {
                node_style = style_sheet.active.inherit_from(&node_style);
            }

            let computed_style = match arena.parent(index).await {
                Some(parent_index) => {
                    node_style.inherit_from(computed_styles.get(&parent_index).unwrap())
                }
                None => node_style.clone(),
            };
            computed_styles.insert(index, computed_style);
            found_nodes.push_back(index);
        }

        for (index, style) in computed_styles {
            effective_styles.insert(index, style.effective_style(scene).await);
        }
        let effective_styles = Arc::new(effective_styles);

        // Traverse the found nodes starting at the back (leaf nodes) and iterate upwards to update stretch
        let mut layout_solvers = HashMap::new();
        while let Some(index) = found_nodes.pop_back() {
            let node = arena.get(index).await.unwrap();
            let effective_style = effective_styles.get(&index).unwrap().clone();
            let mut context = StyledContext::new(
                index,
                scene.clone(),
                effective_style.clone(),
                arena.clone(),
                ui_state.clone(),
            );
            let solver = node.layout(&mut context).await?;
            layout_solvers.insert(index, KludgineHandle::new(solver));
        }

        let layout_data = LayoutEngine::new(layout_solvers, effective_styles.clone(), root);

        while let Some(index) = layout_data.next_to_layout().await {
            let effective_style = effective_styles.get(&index).unwrap().clone();
            let mut context = LayoutContext::new(
                index,
                scene.clone(),
                effective_style.clone(),
                layout_data.clone(),
                arena.clone(),
                ui_state.clone(),
            );
            let computed_layout = match context.layout_for(index).await {
                Some(layout) => layout,
                None => Layout {
                    bounds: Rect::sized(Point::default(), scene.size().await),
                    padding: Surround::default(),
                    margin: Surround::default(),
                },
            };
            context
                .layout_within(index, &computed_layout.inner_bounds())
                .await?;
            let node = arena.get(index).await.unwrap();
            node.set_layout(computed_layout).await;
        }

        let node = arena.get(root).await.unwrap();
        let root_layout = node.last_layout().await;
        layout_data.insert_layout(root, root_layout, false).await;

        Ok(layout_data)
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

    pub async fn effective_styles(&self) -> Arc<HashMap<Index, EffectiveStyle>> {
        let data = self.data.read().await;
        data.effective_styles.clone()
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

impl std::ops::DerefMut for LayoutContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl LayoutContext {
    pub(crate) fn new<I: Into<Index>>(
        index: I,
        scene: SceneTarget,
        effective_style: EffectiveStyle,
        layout: LayoutEngine,
        arena: HierarchicalArena,
        ui_state: UIState,
    ) -> Self {
        Self {
            base: StyledContext::new(index, scene, effective_style, arena, ui_state),
            layout,
        }
    }

    pub async fn clone_for<I: Into<Index>>(&self, index: I) -> Self {
        let index = index.into();
        let effective_style = self.layout.effective_style(&index).await.unwrap();
        Self {
            base: StyledContext::new(
                // TODO use clone_for once it's fixed
                index,
                self.scene().clone(),
                effective_style,
                self.arena().clone(),
                self.ui_state.clone(),
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
