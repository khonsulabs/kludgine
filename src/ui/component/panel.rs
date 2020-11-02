use crate::{
    math::Scaled,
    style::{
        BackgroundColor, ColorPair, GenericStyle, UnscaledFallbackStyle, UnscaledStyleComponent,
    },
    ui::{
        component::{Component, ControlBorder, EntityBuilder, InteractiveComponent, Pane},
        Context, Entity, Layout, StyledContext,
    },
    KludgineResult,
};
use async_trait::async_trait;
use std::fmt::Debug;

use super::ComponentBorder;

#[async_trait]
pub trait PanelProvider: Send + Sync + 'static {
    type Index: Eq + Clone + Debug + Send + Sync;
    type Event: Clone + Debug + Send + Sync;

    async fn initialize_panel(
        &mut self,
        index: &Self::Index,
        context: &mut Context,
    ) -> KludgineResult<()>;

    fn new_entity<T: InteractiveComponent + 'static>(
        &self,
        context: &mut Context,
        component: T,
    ) -> EntityBuilder<T, PanelMessage<Self::Event>> {
        context.insert_new_entity(context.index(), component)
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
        let pane = self.new_entity(context, Pane::default()).insert().await?;

        let mut child_context = context.clone_for(&pane);
        self.provider
            .initialize_panel(&self.current_index, &mut child_context)
            .await?;

        self.set_pane(pane, context).await;

        Ok(())
    }
}

#[async_trait]
impl<T: PanelProvider> Component for Panel<T> {
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        self.recreate_pane(context).await
    }

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        self.render_standard_background::<PanelBackgroundColor, PanelBorder>(context, layout)
            .await
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

#[derive(Debug, Clone)]
pub struct PanelBackgroundColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for PanelBackgroundColor {
    fn unscaled_should_be_inherited(&self) -> bool {
        false
    }
}

impl Default for PanelBackgroundColor {
    fn default() -> Self {
        Self(BackgroundColor::default().0)
    }
}

impl UnscaledFallbackStyle for PanelBackgroundColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style.get::<Self>().cloned().or_else(|| {
            BackgroundColor::lookup_unscaled(style).map(|fg| PanelBackgroundColor(fg.0))
        })
    }
}

impl Into<ColorPair> for PanelBackgroundColor {
    fn into(self) -> ColorPair {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct PanelBorder(pub ComponentBorder);
impl UnscaledStyleComponent<Scaled> for PanelBorder {}

impl UnscaledFallbackStyle for PanelBorder {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlBorder::lookup_unscaled(style).map(|cb| PanelBorder(cb.0)))
    }
}

impl Into<ComponentBorder> for PanelBorder {
    fn into(self) -> ComponentBorder {
        self.0
    }
}
