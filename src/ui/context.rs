use crate::{
    style::Style,
    ui::{global_arena, Component, Entity, Index, Node, NodeData},
    KludgineResult,
};
mod layout_context;
mod scene_context;
mod styled_context;
pub use self::{
    layout_context::{LayoutContext, LayoutEngine},
    scene_context::SceneContext,
    styled_context::StyledContext,
};

pub struct Context {
    index: Index,
}

impl Context {
    pub fn index(&self) -> Index {
        self.index
    }
}

impl Context {
    pub(crate) fn new<I: Into<Index>>(index: I) -> Self {
        Self {
            index: index.into(),
        }
    }

    pub async fn set_parent<I: Into<Index>>(&self, parent: Option<I>) {
        global_arena()
            .set_parent(self.index, parent.map(|p| p.into()))
            .await
    }

    pub async fn add_child<I: Into<Index>>(&self, child: I) {
        let child = child.into();

        global_arena().set_parent(child, Some(self.index)).await
    }

    pub async fn send<T: Component + 'static>(&self, target: Entity<T>, message: T::Message) {
        if let Some(target_node) = global_arena().get(target).await {
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

    pub fn new_entity<T: Component + 'static>(&self, component: T) -> EntityBuilder<T> {
        EntityBuilder {
            component,
            parent: Some(self.index),
            style: Style::default(),
        }
    }

    pub fn clone_for<I: Into<Index>>(&self, index: I) -> Self {
        Self {
            index: index.into(),
        }
    }
}

pub struct EntityBuilder<C> {
    component: C,
    parent: Option<Index>,
    style: Style,
}

impl<C> EntityBuilder<C>
where
    C: Component + 'static,
{
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub async fn insert(self) -> KludgineResult<Entity<C>> {
        let index = {
            let node = Node::new(self.component, self.style);
            let index = global_arena().insert(self.parent, node).await;

            let mut context = Context::new(index);
            global_arena()
                .get(index)
                .await
                .unwrap()
                .initialize(&mut context)
                .await?;

            index
        };
        Ok(Entity {
            index,
            _phantom: std::marker::PhantomData::default(),
        })
    }
}
