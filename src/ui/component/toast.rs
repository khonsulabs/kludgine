use crate::{
    math::{Raw, Scaled, Size, Surround},
    style::{theme::Selector, Style, StyleComponent},
    ui::{
        component::{Component, InteractiveComponent, Label, StandaloneComponent},
        Context, Entity, StyledContext,
    },
    KludgineResult,
};
use async_trait::async_trait;
use euclid::Scale;

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
        let padding = context
            .effective_style()
            .get_or_default::<ToastPadding<Raw>>()
            .0
            / context.scene().scale_factor().await;

        let contraints_minus_padding = padding.inset_constraints(constraints);
        Ok(context
            .content_size(&self.contents.entity(), &contraints_minus_padding)
            .await?
            + padding.minimum_size())
    }

    // TODO override render background?
    // TODO implement timeout for the toast
    // TODO figure out how to let the user control toast placement?
}

impl<C> StandaloneComponent for Toast<C> where C: InteractiveComponent + 'static {}

#[derive(Debug, Clone, Default)]
pub struct ToastPadding<Unit>(pub Surround<f32, Unit>);

impl StyleComponent<Scaled> for ToastPadding<Scaled> {
    fn scale(&self, scale: Scale<f32, Scaled, Raw>, destination: &mut Style<Raw>) {
        destination.push(ToastPadding(self.0 * scale))
    }
}

impl StyleComponent<Raw> for ToastPadding<Raw> {
    fn scale(&self, _scale: Scale<f32, Raw, Raw>, map: &mut Style<Raw>) {
        map.push(ToastPadding(self.0));
    }
}
