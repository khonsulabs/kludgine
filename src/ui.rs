mod arena;
mod component;
mod context;
mod image;
mod layout;
mod node;
pub use self::image::Image;
use crate::{
    math::{Point, Rect},
    scene::SceneTarget,
    style::Style,
    window::InputEvent,
    KludgineHandle, KludgineResult,
};
use arena::{HierarchicalArena, Index};
use component::BaseComponent;
pub use component::{Component, LayoutConstraints};
pub use context::*;
pub use layout::Layout;
pub use node::Node;
pub(crate) use node::NodeData;

pub struct UserInterface {
    arena: KludgineHandle<HierarchicalArena>,
    base_style: Style,
}

impl UserInterface {
    pub fn new(base_style: Style) -> Self {
        let arena = KludgineHandle::new(HierarchicalArena::new());
        Self { arena, base_style }
    }

    pub async fn render(&self, scene: &SceneTarget) -> KludgineResult<()> {
        let layout = Layout::new(self.arena.clone());

        {
            let arena = self.arena.read().await;
            let size = scene.size().await;
            for index in arena.children(&None) {
                let desired_size = layout.measure(index, size).await?;
                // TODO better placement of root nodes
                layout
                    .place(index, Rect::sized(Point::default(), desired_size))
                    .await;
            }
        }
        println!("Done with layout");

        let mut arena = self.arena.write().await;
        for index in arena.iter().collect::<Vec<_>>() {
            let mut context = Context::new(index, self.arena.clone());
            let node = arena.get_mut(index).unwrap();

            if let Some(location) = layout.placement(index).await {
                node.component.render(&mut context, scene, location).await?;
            }
        }

        println!("Done with render");

        Ok(())
    }

    pub async fn update(&mut self, scene: &SceneTarget) -> KludgineResult<()> {
        let mut arena = self.arena.write().await;

        // Loop twice, once to allow all the pending messages to be exhausted across all
        // nodes. Then after all messages have been processed, trigger the update method
        // for each node.

        for index in arena.iter().collect::<Vec<_>>() {
            let mut context = Context::new(index, self.arena.clone());
            let node = arena.get_mut(index).unwrap();

            node.component.process_pending_events(&mut context).await?;
        }

        for index in arena.iter().collect::<Vec<_>>() {
            let mut context = Context::new(index, self.arena.clone());
            let node = arena.get_mut(index).unwrap();

            node.component.update(&mut context, scene).await?;
        }

        Ok(())
    }

    pub async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        let mut arena = self.arena.write().await;

        for index in arena.iter().collect::<Vec<_>>() {
            let mut context = Context::new(index, self.arena.clone());
            let node = arena.get_mut(index).unwrap();

            node.component.process_input(&mut context, event).await?;
        }

        Ok(())
    }

    pub fn new_entity<C: Component + 'static>(&mut self, component: C) -> EntityBuilder<C> {
        EntityBuilder {
            arena: self.arena.clone(),
            component,
            parent: None,
            style: Style::default(),
        }
    }
}

pub struct EntityBuilder<C> {
    arena: KludgineHandle<HierarchicalArena>,
    component: C,
    parent: Option<Index>,
    style: Style,
}

impl<C> EntityBuilder<C>
where
    C: Component + 'static,
{
    pub fn within<I: Into<Index>>(mut self, parent: I) -> Self {
        self.parent = Some(parent.into());
        self
    }

    pub async fn styled(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub async fn insert(self) -> KludgineResult<Entity<C>> {
        let index = {
            let mut arena = self.arena.write().await;
            let node = Node::new(self.component);
            arena.insert(self.parent, node)
        };
        Ok(Entity {
            index,
            _phantom: std::marker::PhantomData::default(),
        })
    }
}

#[derive(Debug, Copy, Clone)]
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
