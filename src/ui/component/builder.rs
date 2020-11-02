use async_handle::Handle;
use generational_arena::Index;

use crate::{
    math::Scaled,
    prelude::Target,
    style::{Style, StyleSheet},
    ui::{
        node::ThreadsafeAnyMap, AbsoluteBounds, Context, Entity, HierarchicalArena, Node, UIState,
    },
    KludgineResult,
};

use super::{Callback, InteractiveComponent};

pub struct EntityBuilder<C, P>
where
    C: InteractiveComponent + 'static,
{
    pub(crate) components: ThreadsafeAnyMap,
    pub(crate) scene: Target,
    pub(crate) parent: Option<Index>,
    pub(crate) style_sheet: StyleSheet,
    pub(crate) interactive: bool,
    pub(crate) callback: Option<Callback<C::Event>>,
    pub(crate) ui_state: UIState,
    pub(crate) arena: HierarchicalArena,
    pub(crate) _marker: std::marker::PhantomData<P>,
}

impl<C, P> EntityBuilder<C, P>
where
    C: InteractiveComponent + 'static,
    P: Send + Sync + 'static,
{
    pub fn style_sheet<S: Into<StyleSheet>>(mut self, sheet: S) -> Self {
        self.style_sheet = sheet.into();
        self
    }

    pub fn normal_style(mut self, style: Style<Scaled>) -> Self {
        self.style_sheet.normal = style;
        self
    }

    pub fn hover(mut self, style: Style<Scaled>) -> Self {
        self.style_sheet.hover = style;
        self
    }

    pub fn active(mut self, style: Style<Scaled>) -> Self {
        self.style_sheet.active = style;
        self
    }

    pub fn focus(mut self, style: Style<Scaled>) -> Self {
        self.style_sheet.focus = style;
        self
    }

    pub fn bounds(mut self, bounds: AbsoluteBounds) -> Self {
        self.components.insert(Handle::new(bounds));
        self
    }

    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    pub fn callback<F: Fn(C::Event) -> P + Send + Sync + 'static>(mut self, callback: F) -> Self {
        let target = Context::new(
            self.parent.unwrap(),
            self.arena.clone(),
            self.ui_state.clone(),
            self.scene.clone(),
        );
        self.callback = Some(Callback::new(target, callback));
        self
    }

    pub async fn insert(mut self) -> KludgineResult<Entity<C>> {
        self.components.insert(Handle::new(self.style_sheet));
        let index = {
            let node = Node::from_components::<C>(self.components, self.interactive, self.callback);
            let index = self.arena.insert(self.parent, node).await;

            let mut context = Context::new(
                index,
                self.arena.clone(),
                self.ui_state.clone(),
                self.scene.clone(),
            );
            self.arena
                .get(&index)
                .await
                .unwrap()
                .initialize(&mut context)
                .await?;

            index
        };
        Ok(Entity::new(Context::new(
            index,
            self.arena.clone(),
            self.ui_state,
            self.scene.clone(),
        )))
    }
}
