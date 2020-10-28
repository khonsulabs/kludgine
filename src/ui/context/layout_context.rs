use crate::{
    math::{Point, Raw, Rect, Scaled, Size, Surround},
    scene::Target,
    style::Style,
    ui::{
        Context, HierarchicalArena, Index, Indexable, Layout, LayoutSolver, StyledContext, UIState,
    },
    Handle, KludgineResult,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct LayoutEngine {
    data: Handle<LayoutEngineData>,
}

#[derive(Debug)]
struct LayoutEngineData {
    layout_solvers: HashMap<Index, Handle<Box<dyn LayoutSolver>>>,
    pub(crate) layouts: HashMap<Index, Layout>,
    indicies_to_process: VecDeque<Index>,
    render_queue: VecDeque<Index>,
    effective_styles: Arc<HashMap<Index, Style<Raw>>>,
}

impl LayoutEngine {
    pub fn new(
        layout_solvers: HashMap<Index, Handle<Box<dyn LayoutSolver>>>,
        effective_styles: Arc<HashMap<Index, Style<Raw>>>,
        root: impl Indexable,
    ) -> Self {
        let mut indicies_to_process = VecDeque::default();
        indicies_to_process.push_back(root.index());
        Self {
            data: Handle::new(LayoutEngineData {
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
        scene: &Target,
        hovered_indicies: HashSet<Index>,
    ) -> KludgineResult<Self> {
        let mut effective_styles = HashMap::new();
        let mut computed_styles = HashMap::new();
        let mut traverser = arena.traverse(&root).await;
        let mut found_nodes = VecDeque::new();
        while let Some(index) = traverser.next().await {
            if let Some(node) = arena.get(&index).await {
                let style_sheet = node.style_sheet().await;
                let mut node_style = style_sheet.normal;

                if hovered_indicies.contains(&index) {
                    node_style = style_sheet.hover.merge_with(&node_style, false);
                }

                if ui_state.focused().await == Some(index) {
                    node_style = style_sheet.focus.merge_with(&node_style, false);
                }

                if ui_state.active().await == Some(index) {
                    node_style = style_sheet.active.merge_with(&node_style, false);
                }

                let computed_style = match arena.parent(index).await {
                    Some(parent_index) => {
                        node_style.merge_with(computed_styles.get(&parent_index).unwrap(), true)
                    }
                    None => node_style.clone(),
                };
                computed_styles.insert(index, computed_style);
                found_nodes.push_back(index);
            }
        }

        for (index, style) in computed_styles {
            effective_styles.insert(index, style.effective_style(scene).await);
        }
        let effective_styles = Arc::new(effective_styles);

        // Traverse the found nodes starting at the back (leaf nodes) and iterate upwards to update stretch
        let mut layout_solvers = HashMap::new();
        while let Some(index) = found_nodes.pop_back() {
            if let Some(node) = arena.get(&index).await {
                let mut context = StyledContext::new(
                    index,
                    scene.clone(),
                    effective_styles.clone(),
                    arena.clone(),
                    ui_state.clone(),
                );
                let solver = node.layout(&mut context).await?;
                layout_solvers.insert(index, Handle::new(solver));
            }
        }

        let layout_data = LayoutEngine::new(layout_solvers, effective_styles.clone(), root);

        while let Some(index) = layout_data.next_to_layout().await {
            let mut context = LayoutContext::new(
                index,
                scene.clone(),
                effective_styles.clone(),
                layout_data.clone(),
                arena.clone(),
                ui_state.clone(),
            );
            let computed_layout = match context.layout_for(index).await {
                Some(layout) => layout,
                None => {
                    let layout = Layout {
                        bounds: Rect::new(Point::default(), scene.size().await),
                        padding: Surround::default(),
                        margin: Surround::default(),
                    };
                    layout_data
                        .insert_layout(index, layout.clone(), false)
                        .await;
                    layout
                }
            };
            context
                .layout_within(index, &computed_layout.inner_bounds())
                .await?;

            if let Some(node) = arena.get(&index).await {
                node.set_layout(computed_layout).await;
            }
        }

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

    pub async fn effective_style(&self, index: &Index) -> Option<Style<Raw>> {
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
        bounds: &Rect<f32, Scaled>,
        content_size: &Size<f32, Scaled>,
    ) -> KludgineResult<()> {
        let solver_handle = {
            let data = self.data.read().await;
            data.layout_solvers.get(index).cloned()
        };
        if let Some(solver_handle) = solver_handle {
            let solver = solver_handle.read().await;
            solver.layout_within(bounds, content_size, context).await?;
        }
        Ok(())
    }

    pub async fn effective_styles(&self) -> Arc<HashMap<Index, Style<Raw>>> {
        let data = self.data.read().await;
        data.effective_styles.clone()
    }
}

pub struct LayoutContext {
    base: StyledContext,
    layout: LayoutEngine,
}

impl std::ops::Deref for LayoutContext {
    type Target = Context;

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
    pub(crate) fn new<I: Indexable>(
        index: I,
        scene: Target,
        effective_styles: Arc<HashMap<Index, Style<Raw>>>,
        layout: LayoutEngine,
        arena: HierarchicalArena,
        ui_state: UIState,
    ) -> Self {
        Self {
            base: StyledContext::new(index, scene, effective_styles, arena, ui_state),
            layout,
        }
    }

    pub async fn clone_for<I: Indexable>(&self, index: &I) -> Self {
        let index = index.index();
        Self {
            base: self.base.clone_for(&index),
            layout: self.layout.clone(),
        }
    }

    pub fn styled_context(&mut self) -> &'_ mut StyledContext {
        &mut self.base
    }

    pub async fn layout_within(
        &mut self,
        index: impl Indexable,
        bounds: &Rect<f32, Scaled>,
    ) -> KludgineResult<()> {
        let index = index.index();
        if let Some(node) = self.arena().get(&index).await {
            let content_size = node
                .content_size(
                    self.styled_context(),
                    &Size::new(Some(bounds.size.width), Some(bounds.size.height)),
                )
                .await?;

            let mut solving_context = self.clone_for(&index).await;
            self.layout
                .solve_layout_for(&index, &mut solving_context, bounds, &content_size)
                .await?;
        }

        Ok(())
    }

    pub async fn layout_for(&self, index: impl Indexable) -> Option<Layout> {
        self.layout.get_layout(&index.index()).await
    }

    pub async fn insert_layout(&self, index: impl Indexable, layout: Layout) {
        let index = index.index();
        self.layout.insert_layout(index, layout, true).await;
    }
}
