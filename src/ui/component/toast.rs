use crate::{
    math::{Scaled, Size},
    runtime::Runtime,
    style::theme::Selector,
    ui::{
        component::{
            pending::PendingComponent, Component, InteractiveComponent, InteractiveComponentExt,
            Label, StandaloneComponent,
        },
        Context, StyledContext,
    },
    KludgineResult, RequiresInitialization,
};
use async_lock::Mutex;
use async_trait::async_trait;
use generational_arena::Index;
use instant::Instant;
use once_cell::sync::OnceCell;
use std::time::Duration;

static ACTIVE_TOASTS: OnceCell<Mutex<Vec<Index>>> = OnceCell::new();

pub struct Toast<C>
where
    C: InteractiveComponent + 'static,
{
    index: Option<Index>,
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
            index: None,
        }
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
        self.index = Some(context.index());
        let mut active_toasts = ACTIVE_TOASTS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .await;
        for toast_layer in context
            .layers()
            .await
            .into_iter()
            .filter(|l| active_toasts.contains(&l.root))
        {
            context.remove(&toast_layer.root).await;
        }
        active_toasts.push(context.index());

        if let PendingComponent::Pending(contents) = std::mem::replace(
            &mut self.contents,
            PendingComponent::Entity(Default::default()),
        ) {
            self.contents =
                PendingComponent::Entity(self.new_entity(context, contents).await?.insert().await?);
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

impl<C> Drop for Toast<C>
where
    C: InteractiveComponent + 'static,
{
    fn drop(&mut self) {
        if let Some(index) = self.index {
            Runtime::spawn(async move {
                let mut active_toasts = ACTIVE_TOASTS.get().unwrap().lock().await;
                active_toasts.retain(|i| *i != index);
            });
        }
    }
}
