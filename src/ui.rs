mod arena;
mod component;
mod context;
mod image;
mod node;
mod placements;
pub use self::image::Image;
use crate::{
    math::{Point, Rect},
    scene::SceneTarget,
    style::{Layout, Style},
    window::InputEvent,
    KludgineHandle, KludgineResult,
};
use arena::{HierarchicalArena, Index};
pub(crate) use component::BaseComponent;
pub use component::{Component, LayoutConstraints};
pub use context::*;
pub use node::Node;
pub(crate) use node::NodeData;
pub use placements::Placements;
use std::collections::HashMap;

#[derive(Default)]
pub struct UserInterface {
    pub(crate) arena: KludgineHandle<HierarchicalArena>,
}

impl UserInterface {
    pub async fn render(&self, scene: &SceneTarget) -> KludgineResult<()> {
        let layout = Placements::new(self.arena.clone());
        let mut effective_styles = HashMap::new();

        {
            let arena = self.arena.read().await;
            let mut computed_styles = HashMap::new();
            for index in arena.iter() {
                let node_style = arena.get(index).as_ref().unwrap().style().await;
                let computed_style = match arena.parent(index) {
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
            for index in arena.iter() {
                let parent_bounds = match arena.parent(index) {
                    Some(parent) => layout.placement(parent).await.unwrap(),
                    None => Rect::sized(Point::default(), size),
                };
                layout
                    .place(index, parent_bounds, effective_styles.get(&index).unwrap())
                    .await?;
            }
        }

        let arena = self.arena.read().await;
        for (index, node) in arena
            .iter()
            .map(|index| (index, arena.get(index).unwrap()))
            .collect::<Vec<_>>()
        {
            let mut context = Context::new(index, self.arena.clone());
            let location = layout.placement(index).await.unwrap();

            node.render(
                &mut context,
                scene,
                location,
                effective_styles.get(&index).unwrap(),
            )
            .await?;
        }

        Ok(())
    }

    pub async fn update(&mut self, scene: &SceneTarget) -> KludgineResult<()> {
        let arena = self.arena.read().await;

        // Loop twice, once to allow all the pending messages to be exhausted across all
        // nodes. Then after all messages have been processed, trigger the update method
        // for each node.

        for index in arena.iter().collect::<Vec<_>>() {
            let mut context = Context::new(index, self.arena.clone());
            let node = arena.get(index).unwrap();

            node.process_pending_events(&mut context).await?;
        }

        for index in arena.iter().collect::<Vec<_>>() {
            let mut context = Context::new(index, self.arena.clone());
            let node = arena.get(index).unwrap();

            node.update(&mut context, scene).await?;
        }

        Ok(())
    }

    pub async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        let arena = self.arena.read().await;

        for index in arena.iter().collect::<Vec<_>>() {
            let mut context = Context::new(index, self.arena.clone());
            let node = arena.get(index).unwrap();

            node.process_input(&mut context, event).await?;
        }

        Ok(())
    }

    async fn initialize(&self, index: Index) -> KludgineResult<()> {
        let node = {
            let arena = self.arena.read().await;
            arena.get(index).unwrap()
        };

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
            let mut arena = self.arena.write().await;
            let node = Node::new(component, base_style, base_layout);

            arena.insert(None, node)
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
