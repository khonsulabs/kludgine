mod arena;
mod component;
mod context;
mod node;
mod sprite;
use crate::{scene::SceneTarget, style::Style, window::InputEvent, KludgineHandle, KludgineResult};
use arena::{HierarchicalArena, Index};
use component::BaseComponent;
pub use component::{Component, LayoutConstraints};
pub use context::*;
pub use node::Node;
pub use sprite::Image;

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
        todo!()
    }

    pub async fn update(&mut self, scene: &SceneTarget) -> KludgineResult<()> {
        let mut arena = self.arena.write().await;

        for index in arena.iter().collect::<Vec<_>>() {
            let mut context = Context::new(index, self.arena.clone(), scene.clone());
            let node = arena.get_mut(index).unwrap();

            node.component.update(&mut context).await?;
        }

        Ok(())
    }

    pub async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        todo!()
    }

    pub fn new_entity<C: Component + 'static>(&mut self, component: C) -> EntityBuilder<C> {
        EntityBuilder {
            arena: self.arena.clone(),
            component,
            parent: None,
        }
    }
}

pub struct EntityBuilder<C> {
    arena: KludgineHandle<HierarchicalArena>,
    component: C,
    parent: Option<Index>,
}

impl<C> EntityBuilder<C>
where
    C: Component + 'static,
{
    pub fn within<I: Into<Index>>(mut self, parent: I) -> Self {
        self.parent = Some(parent.into());
        self
    }

    pub async fn insert(self) -> KludgineResult<Entity<C>> {
        let index = {
            let mut arena = self.arena.write().await;
            let node = Node {
                component: Box::new(self.component),
            };
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
