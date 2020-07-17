use crate::{
    style::{Layout, Style},
    ui::{Component, Entity, HierarchicalArena, Index, Node, NodeData},
    KludgineHandle, KludgineResult,
};
mod scene_context;
mod styled_context;
pub use self::{scene_context::SceneContext, styled_context::StyledContext};

pub struct Context {
    index: Index,
    arena: KludgineHandle<HierarchicalArena>,
}

impl Context {
    pub fn index(&self) -> Index {
        self.index
    }
}

impl Context {
    pub(crate) fn new<I: Into<Index>>(index: I, arena: KludgineHandle<HierarchicalArena>) -> Self {
        Self {
            index: index.into(),
            arena,
        }
    }

    pub async fn set_parent<I: Into<Index>>(&self, parent: Option<I>) {
        let mut arena = self.arena.write().await;
        arena.set_parent(self.index, parent.map(|p| p.into()))
    }

    pub async fn add_child<I: Into<Index>>(&self, child: I) {
        let child = child.into();
        let mut arena = self.arena.write().await;
        arena.set_parent(child, Some(self.index))
    }

    pub async fn send<T: Component + 'static>(&self, target: Entity<T>, message: T::Message) {
        let arena = self.arena.read().await;
        if let Some(target_node) = arena.get(target) {
            let component = target_node.component.read().await;
            if let Some(node_data) = component.as_any().downcast_ref::<NodeData<T>>() {
                node_data
                    .sender
                    .send(message)
                    .expect("Error sending to component");
            } else {
                unreachable!("Invalid type in Entity<T> -- Node contained different type than T")
            }
        }
    }

    pub async fn layout(&self) -> Layout {
        let arena = self.arena.read().await;
        arena.get(self.index).unwrap().layout().await
    }

    pub fn new_entity<T: Component + 'static>(&self, component: T) -> EntityBuilder<T> {
        EntityBuilder {
            arena: self.arena.clone(),
            component,
            parent: Some(self.index),
            style: Style::default(),
            layout: Layout::default(),
        }
    }

    pub fn clone_for<I: Into<Index>>(&self, index: I) -> Self {
        Self {
            index: index.into(),
            arena: self.arena.clone(),
        }
    }
}

pub struct EntityBuilder<C> {
    arena: KludgineHandle<HierarchicalArena>,
    component: C,
    parent: Option<Index>,
    style: Style,
    layout: Layout,
}

impl<C> EntityBuilder<C>
where
    C: Component + 'static,
{
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    pub async fn insert(self) -> KludgineResult<Entity<C>> {
        let index = {
            let mut arena = self.arena.write().await;
            let node = Node::new(self.component, self.style, self.layout);
            let index = arena.insert(self.parent, node);

            let mut context = Context::new(index, self.arena.clone());
            arena.get(index).unwrap().initialize(&mut context).await?;

            index
        };
        Ok(Entity {
            index,
            _phantom: std::marker::PhantomData::default(),
        })
    }
}
