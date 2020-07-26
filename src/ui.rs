mod arena;
mod component;
mod context;
mod image;
mod label;
mod layout;
mod node;

pub(crate) use self::{component::BaseComponent, node::NodeData};
pub use self::{
    component::{Component, LayoutConstraints},
    context::*,
    image::Image,
    label::Label,
    layout::*,
    node::Node,
};
use crate::{
    math::{Point, Rect, Surround},
    runtime::Runtime,
    scene::SceneTarget,
    style::Style,
    window::InputEvent,
    KludgineHandle, KludgineResult,
};
use arena::{HierarchicalArena, Index};
use once_cell::sync::OnceCell;
use std::collections::{HashMap, VecDeque};

static UI: OnceCell<HierarchicalArena> = OnceCell::new();

pub(crate) fn global_arena() -> &'static HierarchicalArena {
    UI.get_or_init(HierarchicalArena::default)
}

pub struct UserInterface<C>
where
    C: Component + 'static,
{
    pub(crate) root: Entity<C>,
}

impl<C> UserInterface<C>
where
    C: Component + 'static,
{
    pub async fn new(root: C) -> KludgineResult<Self> {
        let root = Entity::new({
            let node = Node::new(root, Style::default());

            global_arena().insert(None, node).await
        });

        let ui = Self { root };
        ui.initialize(root).await?;
        Ok(ui)
    }

    pub async fn render(&mut self, scene: &SceneTarget) -> KludgineResult<()> {
        let mut effective_styles = HashMap::new();

        let layouts = {
            let mut computed_styles = HashMap::new();
            let mut traverser = global_arena().traverse(self.root).await;
            let mut found_nodes = VecDeque::new();
            while let Some(index) = traverser.next().await {
                let node_style = global_arena()
                    .get(index)
                    .await
                    .as_ref()
                    .unwrap()
                    .style()
                    .await;
                let computed_style = match global_arena().parent(index).await {
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

            // Traverse the found nodes starting at the back (leaf nodes) and iterate upwards to update stretch
            let mut layout_solvers = HashMap::new();
            while let Some(index) = found_nodes.pop_back() {
                let node = global_arena().get(index).await.unwrap();
                let effective_style = effective_styles.get(&index).unwrap().clone();
                let mut context = StyledContext::new(index, scene.clone(), effective_style.clone());
                let solver = node.layout(&mut context).await?;
                layout_solvers.insert(index, KludgineHandle::new(solver));
            }

            let layout_data = LayoutEngine::new(
                layout_solvers,
                effective_styles.clone(), // TODO don't really want to clone here
                self.root.index,
            );

            while let Some(index) = layout_data.next_to_layout().await {
                let effective_style = effective_styles.get(&index).unwrap().clone();
                let mut context = LayoutContext::new(
                    index,
                    scene.clone(),
                    effective_style.clone(),
                    layout_data.clone(),
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
            }

            layout_data
        };

        while let Some(index) = layouts.next_to_render().await {
            if let Some(layout) = layouts.get_layout(&index).await {
                let node = global_arena().get(index).await.unwrap();
                let mut context = StyledContext::new(
                    index,
                    scene.clone(),
                    effective_styles.get(&index).unwrap().clone(),
                );
                node.render_background(&mut context, &layout).await?;
                node.render(&mut context, &layout).await?;
            }
        }

        Ok(())
    }

    pub async fn update(&mut self, scene: &SceneTarget) -> KludgineResult<()> {
        // Loop twice, once to allow all the pending messages to be exhausted across all
        // nodes. Then after all messages have been processed, trigger the update method
        // for each node.

        let mut traverser = global_arena().traverse(self.root).await;
        while let Some(index) = traverser.next().await {
            let mut context = Context::new(index);
            let node = global_arena().get(index).await.unwrap();

            node.process_pending_events(&mut context).await?;
        }

        let mut traverser = global_arena().traverse(self.root).await;
        while let Some(index) = traverser.next().await {
            let mut context = SceneContext::new(index, scene.clone());
            let node = global_arena().get(index).await.unwrap();

            node.update(&mut context).await?;
        }

        Ok(())
    }

    pub async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        let mut traverser = global_arena().traverse(self.root).await;
        while let Some(index) = traverser.next().await {
            let mut context = Context::new(index);
            let node = global_arena().get(index).await.unwrap();

            node.process_input(&mut context, event).await?;
        }

        Ok(())
    }

    async fn initialize(&self, index: impl Into<Index>) -> KludgineResult<()> {
        let index = index.into();
        let node = global_arena().get(index).await.unwrap();

        node.initialize(&mut Context::new(index)).await
    }
}

impl<C> Drop for UserInterface<C>
where
    C: Component + 'static,
{
    fn drop(&mut self) {
        let root = self.root;
        Runtime::spawn(async move {
            global_arena().remove(root).await;
        });
    }
}

#[derive(Debug)]
pub struct Entity<C> {
    index: Index,
    _phantom: std::marker::PhantomData<C>,
}

impl<C> Into<Index> for Entity<C> {
    fn into(self) -> Index {
        self.index
    }
}

impl<C> Entity<C> {
    pub fn new(index: Index) -> Self {
        Self {
            index,
            _phantom: std::marker::PhantomData::default(),
        }
    }
}

impl<C> Clone for Entity<C> {
    fn clone(&self) -> Self {
        Self::new(self.index)
    }
}

impl<C> Copy for Entity<C> {}
