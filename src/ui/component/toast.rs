use crate::{
    math::{Scaled, Size},
    style::theme::Selector,
    ui::{
        component::{Component, InteractiveComponent, Label, StandaloneComponent},
        Context, Entity, Layout, StyledContext,
    },
    KludgineResult,
};
use async_trait::async_trait;

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
}

impl<C> Toast<C>
where
    C: InteractiveComponent + 'static,
{
    pub fn new(contents: C) -> Self {
        Self {
            contents: PendingComponent::Pending(contents),
        }
    }

    pub async fn open(self, context: &mut Context) -> KludgineResult<Entity<Self>> {
        context.push_layer(self).await
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
                PendingComponent::Entity(self.new_entity(context, contents).insert().await?);
        } else {
            unreachable!("A component should never be re-initialized");
        }

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

    // async fn render_background(
    //     &self,
    //     context: &mut StyledContext,
    //     _layout: &Layout,
    // ) -> KludgineResult<()> {
    //     dbg!(layout);
    //     let layout = context.last_layout_for(self.contents.entity()).await;
    //     dbg!(layout);
    //     Ok(())
    // }

    async fn render(&mut self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        dbg!(layout);
        Ok(())
    }
    // TODO implement timeout for the toast
    // TODO figure out how to let the user control toast placement?
}

impl<C> StandaloneComponent for Toast<C> where C: InteractiveComponent + 'static {}
