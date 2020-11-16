use crate::{
    ui::{Context, Entity, Indexable},
    KludgineResult,
};
use async_trait::async_trait;
use generational_arena::Index;

use super::InteractiveComponent;

#[derive(Debug)]
pub enum PendingComponent<C> {
    Pending(C),
    Entity(Entity<C>),
}

impl<C> PendingComponent<C> {
    pub fn entity(&self) -> Entity<C> {
        if let PendingComponent::Entity(entity) = self {
            entity.clone()
        } else {
            panic!("Component hasn't been inserted yet.")
        }
    }
}

#[async_trait]
pub trait AnonymousPendingComponent: Send + Sync {
    async fn insert(&mut self, context: &mut Context) -> KludgineResult<Index>;
}

#[async_trait]
impl<C> AnonymousPendingComponent for PendingComponent<C>
where
    C: InteractiveComponent + 'static,
{
    async fn insert(&mut self, context: &mut Context) -> KludgineResult<Index> {
        if let PendingComponent::Pending(contents) =
            std::mem::replace(self, PendingComponent::Entity(Entity::default()))
        {
            Ok(context
                .insert_new_entity::<_, _, ()>(context.index(), contents)
                .await
                .insert()
                .await?
                .index())
        } else {
            unreachable!("A component should never be re-initialized");
        }
    }
}
