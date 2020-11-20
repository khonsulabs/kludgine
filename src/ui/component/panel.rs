use crate::{
    style::theme::Selector,
    ui::{
        component::{Component, EntityBuilder, InteractiveComponent, Pane},
        Context, Entity,
    },
    KludgineResult,
};
use async_trait::async_trait;
use std::fmt::Debug;

use super::InteractiveComponentExt;

#[async_trait]
pub trait PanelProvider: Send + Sync + Sized + 'static {
    type Index: Eq + Clone + Debug + Send + Sync;
    type Event: Clone + Debug + Send + Sync;

    async fn initialize_panel(
        &mut self,
        index: &Self::Index,
        context: &mut Context,
        panel: &Entity<Panel<Self>>,
    ) -> KludgineResult<()>;

    async fn new_entity<T: InteractiveComponent + 'static>(
        &self,
        context: &mut Context,
        component: T,
    ) -> EntityBuilder<T, ()> {
        context.insert_new_entity(context.index(), component).await
    }

    fn entity(&self, context: &mut Context) -> Entity<Pane> {
        context.entity()
    }
}

pub struct Panel<T: PanelProvider> {
    provider: T,
    current_index: T::Index,
    current_pane: Option<Entity<Pane>>,
}

impl<T: PanelProvider> Panel<T> {
    pub fn new(provider: T, initial_index: T::Index) -> Self {
        Self {
            provider,
            current_index: initial_index,
            current_pane: Default::default(),
        }
    }

    async fn set_pane(&mut self, pane: Entity<Pane>, context: &mut Context) {
        if let Some(existing_pane) = &self.current_pane {
            context.remove(existing_pane).await;
        }

        self.current_pane = Some(pane);
    }

    async fn recreate_pane(&mut self, context: &mut Context) -> KludgineResult<()> {
        let pane = self
            .new_entity(context, Pane::default())
            .await
            .insert()
            .await?;

        let mut child_context = context.clone_for(&pane);
        self.provider
            .initialize_panel(
                &self.current_index,
                &mut child_context,
                &self.entity(context),
            )
            .await?;

        self.set_pane(pane, context).await;

        Ok(())
    }
}

#[async_trait]
impl<T: PanelProvider> Component for Panel<T> {
    fn classes(&self) -> Option<Vec<Selector>> {
        Some(vec![Selector::from("panel")])
    }

    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        self.recreate_pane(context).await
    }
}

#[async_trait]
impl<T: PanelProvider> InteractiveComponent for Panel<T> {
    type Message = PanelMessage<T::Event>;
    type Command = PanelCommand<T::Index>;
    type Event = PanelEvent<T::Event>;

    async fn receive_command(
        &mut self,
        context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        match command {
            PanelCommand::SetIndex(index) => {
                if self.current_index != index {
                    self.current_index = index;
                    self.recreate_pane(context).await?;
                }
            }
        }
        Ok(())
    }

    async fn receive_message(
        &mut self,
        context: &mut Context,
        message: Self::Message,
    ) -> KludgineResult<()> {
        let PanelMessage::ChildEvent(message) = message;
        self.callback(context, PanelEvent::ChildEvent(message))
            .await;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum PanelCommand<Index> {
    SetIndex(Index),
}

#[derive(Debug, Clone)]
pub enum PanelEvent<Event> {
    ChildEvent(Event),
}

#[derive(Debug, Clone)]
pub enum PanelMessage<Event> {
    ChildEvent(Event),
}
