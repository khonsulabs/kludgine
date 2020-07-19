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
    scene::SceneTarget,
    style::{Layout, Style},
    window::InputEvent,
    KludgineResult,
};
use arena::{HierarchicalArena, Index};
use std::collections::HashMap;

#[derive(Default)]
pub struct UserInterface {
    pub(crate) arena: HierarchicalArena,
}

impl UserInterface {
    pub async fn render(&self, scene: &SceneTarget) -> KludgineResult<()> {
        let layout = Placements::new(self.arena.clone());
        let mut effective_styles = HashMap::new();

        {
            let mut computed_styles = HashMap::new();
            let mut traverser = self.arena.traverse().await;
            while let Some(index) = traverser.next().await {
                let node_style = self.arena.get(index).await.as_ref().unwrap().style().await;
                let computed_style = match self.arena.parent(index).await {
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
            let mut traverser = self.arena.traverse().await;
            while let Some(index) = traverser.next().await {
                let parent_bounds = match self.arena.parent(index).await {
                    Some(parent) => layout.placement(parent).await.unwrap(),
                    None => Rect::sized(Point::default(), size),
                };
                let mut context = StyledContext::new(
                    index,
                    self.arena.clone(),
                    scene.clone(),
                    effective_styles.get(&index).unwrap().clone(),
                );
                layout.place(index, &parent_bounds, &mut context).await?;
            }
        }

        let mut traverser = self.arena.traverse().await;
        while let Some(index) = traverser.next().await {
            let node = self.arena.get(index).await.unwrap();
            let mut context = StyledContext::new(
                index,
                self.arena.clone(),
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

        let mut traverser = self.arena.traverse().await;
        while let Some(index) = traverser.next().await {
            let mut context = Context::new(index, self.arena.clone());
            let node = self.arena.get(index).await.unwrap();

            node.process_pending_events(&mut context).await?;
        }

        let mut traverser = self.arena.traverse().await;
        while let Some(index) = traverser.next().await {
            let mut context = SceneContext::new(index, self.arena.clone(), scene.clone());
            let node = self.arena.get(index).await.unwrap();

            node.update(&mut context).await?;
        }

        Ok(())
    }

    pub async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        let mut traverser = self.arena.traverse().await;
        while let Some(index) = traverser.next().await {
            let mut context = Context::new(index, self.arena.clone());
            let node = self.arena.get(index).await.unwrap();

            node.process_input(&mut context, event).await?;
        }

        Ok(())
    }

    async fn initialize(&self, index: Index) -> KludgineResult<()> {
        let node = self.arena.get(index).await.unwrap();

        node.initialize(&mut Context::new(index, self.arena.clone()))
            .await
    }

    pub async fn register_root<C: Component + 'static>(
        &self,
        component: C,
        base_style: Style,
        base_layout: Layout,
    ) -> KludgineResult<Entity<C>> {
        let index = {
            let node = Node::new(component, base_style, base_layout);

            self.arena.insert(None, node).await
        };

        self.initialize(index).await?;

        Ok(Entity {
            index,
            _phantom: std::marker::PhantomData::default(),
        })
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
