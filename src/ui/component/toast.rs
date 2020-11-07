use std::time::{Duration, Instant};

use crate::{
    math::{Scaled, Size},
    style::theme::Selector,
    ui::{
        component::{Component, InteractiveComponent, Label, StandaloneComponent},
        Context, Entity, StyledContext,
    },
    KludgineResult, RequiresInitialization,
};
use async_trait::async_trait;

use super::InteractiveComponentExt;

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

pub struct Toast<C>
where
    C: InteractiveComponent,
{
    contents: PendingComponent<C>,
    duration: RequiresInitialization<Duration>,
    target_time: RequiresInitialization<Instant>,
}

impl<C> Toast<C>
where
    C: InteractiveComponent + 'static,
{
    pub fn new(contents: C) -> Self {
        Self {
            contents: PendingComponent::Pending(contents),
            target_time: Default::default(),
            duration: Default::default(),
        }
    }

    pub async fn open(self, context: &mut Context) -> KludgineResult<Entity<Self>> {
        context.new_layer(self).insert().await
    }
}

impl Toast<Label> {
    pub fn text(contents: String) -> Self {
        Self::new(Label::new(contents))
    }
}

#[async_trait]
impl<C> Component for Toast<C>
where
    C: InteractiveComponent + 'static,
{
    fn classes(&self) -> Option<Vec<Selector>> {
        Some(vec![Selector::from("toast")])
    }

    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        if let PendingComponent::Pending(contents) = std::mem::replace(
            &mut self.contents,
            PendingComponent::Entity(Default::default()),
        ) {
            self.contents =
                PendingComponent::Entity(self.new_entity(context, contents).await.insert().await?);
        } else {
            unreachable!("A component should never be re-initialized");
        }

        let duration = self.component::<Duration>(context).await;
        self.duration
            .initialize_with(if let Some(duration) = duration {
                let duration = duration.read().await;
                *duration
            } else {
                Duration::from_secs_f32(2.)
            });

        self.target_time
            .initialize_with(Instant::now().checked_add(*self.duration).unwrap());

        Ok(())
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let (content_size, padding) = context
            .content_size_with_padding(&self.contents.entity(), &constraints)
            .await?;
        Ok(content_size + padding.minimum_size())
    }

    async fn update(&mut self, context: &mut Context) -> KludgineResult<()> {
        if Instant::now() > *self.target_time {
            context.remove(&context.index()).await;
        }

        Ok(())
    }
}

impl<C> StandaloneComponent for Toast<C> where C: InteractiveComponent + 'static {}
