use crate::ui::Entity;

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
