use crate::{
    ui::{Component, Entity, HierarchicalArena, Index, NodeData},
    KludgineHandle,
};

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

    pub async fn parent<T: Component + 'static>(&self) -> Option<Entity<T>> {
        let arena = self.arena.read().await;
        if let Some(parent) = arena.parent(self.index) {
            if let Some(node) = arena.get(self.index) {
                if node.component.as_any().is::<NodeData<T>>() {
                    return Some(Entity::new(parent));
                }
            }
        }
        None
    }

    pub async fn send<T: Component + 'static>(&self, target: Entity<T>, message: T::Message) {
        let mut arena = self.arena.write().await;
        if let Some(target_node) = arena.get_mut(target) {
            if let Some(node_data) = target_node.component.as_any().downcast_ref::<NodeData<T>>() {
                node_data
                    .sender
                    .send(message)
                    .expect("Error sending to component");
            } else {
                unreachable!("Invalid type in Entity<T> -- Node contained different type than T")
            }
        }
    }
}
