mod arena;
mod component;
mod context;
mod image;
mod label;
mod node;
mod placements;
pub(crate) use self::{component::BaseComponent, node::NodeData};
pub use self::{
    component::{Component, LayoutConstraints},
    context::*,
    image::Image,
    label::Label,
    node::Node,
    placements::Placements,
};
use crate::{
    math::{Point, Rect},
    runtime::Runtime,
    scene::SceneTarget,
    style::{Layout, Style},
    window::InputEvent,
    KludgineResult,
};
use arena::{HierarchicalArena, Index};
use once_cell::sync::OnceCell;
use std::collections::HashMap;

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
            let node = Node::new(root, Style::default(), Layout::default());

            global_arena().insert(None, node).await
        });

        let ui = Self { root };
        ui.initialize(root).await?;
        Ok(ui)
    }

    pub async fn render(&self, scene: &SceneTarget) -> KludgineResult<()> {
        let layout = Placements::new(global_arena().clone());
        let mut effective_styles = HashMap::new();

        {
            let mut computed_styles = HashMap::new();
            let mut traverser = global_arena().traverse(self.root).await;
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
            }

            for (index, style) in computed_styles {
                effective_styles.insert(index, style.effective_style(scene).await);
            }

            let size = scene.size().await;
            let mut traverser = global_arena().traverse(self.root).await;
            while let Some(index) = traverser.next().await {
                let parent_bounds = match global_arena().parent(index).await {
                    Some(parent) => layout.placement(parent).await.unwrap(),
                    None => Rect::sized(Point::default(), size),
                };
                let mut context = StyledContext::new(
                    index,
                    scene.clone(),
                    effective_styles.get(&index).unwrap().clone(),
                );
                layout.place(index, &parent_bounds, &mut context).await?;
            }
        }

        let mut traverser = global_arena().traverse(self.root).await;
        while let Some(index) = traverser.next().await {
            let node = global_arena().get(index).await.unwrap();
            let mut context = StyledContext::new(
                index,
                scene.clone(),
                effective_styles.get(&index).unwrap().clone(),
            );
            let location = layout.placement(index).await.unwrap();

            node.render(&mut context, &location).await?;
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
