use crate::{
    math::{Raw, Scaled, Size, Surround},
    style::{
        ColorPair, FallbackStyle, GenericStyle, Style, StyleComponent, UnscaledFallbackStyle,
        UnscaledStyleComponent,
    },
    ui::{
        component::{
            Component, ComponentBorder, ControlBackgroundColor, ControlBorder, ControlPadding,
            ControlTextColor, InteractiveComponent, Label, StandaloneComponent,
        },
        Context, Entity, Layout, StyledContext,
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
        let padding = ToastPadding::<Raw>::lookup(context.effective_style())
            .unwrap_or_default()
            .0
            / context.scene().scale_factor().await;

        let contraints_minus_padding = padding.inset_constraints(constraints);
        Ok(context
            .content_size(&self.contents.entity(), &contraints_minus_padding)
            .await?
            + padding.minimum_size())
    }

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        self.render_standard_background::<ToastBackgroundColor, ToastBorder>(context, layout)
            .await
    }

    // TODO override render background
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

impl FallbackStyle<Scaled> for ToastPadding<Scaled> {
    fn lookup(style: &Style<Scaled>) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlPadding::<Scaled>::lookup(style).map(|cp| ToastPadding(cp.0)))
    }
}

impl FallbackStyle<Raw> for ToastPadding<Raw> {
    fn lookup(style: &Style<Raw>) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlPadding::<Raw>::lookup(style).map(|cp| ToastPadding(cp.0)))
    }
}

#[derive(Debug, Clone)]
pub struct ToastBackgroundColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for ToastBackgroundColor {}

impl Default for ToastBackgroundColor {
    fn default() -> Self {
        Self(ControlBackgroundColor::default().0)
    }
}

impl UnscaledFallbackStyle for ToastBackgroundColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style.get::<Self>().cloned().or_else(|| {
            ControlBackgroundColor::lookup_unscaled(style).map(|fg| ToastBackgroundColor(fg.0))
        })
    }
}

impl Into<ColorPair> for ToastBackgroundColor {
    fn into(self) -> ColorPair {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct ToastTextColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for ToastTextColor {}

impl Default for ToastTextColor {
    fn default() -> Self {
        Self(ControlTextColor::default().0)
    }
}

impl UnscaledFallbackStyle for ToastTextColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlTextColor::lookup_unscaled(style).map(|fg| ToastTextColor(fg.0)))
    }
}

impl Into<ColorPair> for ToastTextColor {
    fn into(self) -> ColorPair {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct ToastBorder(pub ComponentBorder);
impl UnscaledStyleComponent<Scaled> for ToastBorder {}

impl UnscaledFallbackStyle for ToastBorder {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlBorder::lookup_unscaled(style).map(|cb| ToastBorder(cb.0)))
    }
}

impl Into<ComponentBorder> for ToastBorder {
    fn into(self) -> ComponentBorder {
        self.0
    }
}
