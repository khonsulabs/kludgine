mod animation;
mod arena;
mod button;
mod component;
mod context;
mod control;
mod image;
mod label;
mod layout;
mod node;

pub(crate) use self::node::NodeData;
pub use self::{
    animation::AnimationCanvas,
    button::{Button, ButtonStyle},
    component::{
        AnimatableComponent, Callback, Component, EntityBuilder, EventStatus, InteractiveComponent,
        LayoutConstraints, StandaloneComponent,
    },
    context::*,
    control::ControlEvent,
    image::{Image, ImageCommand, ImageOptions, ImageScaling},
    label::{Label, LabelCommand},
    layout::*,
    node::{Node, NodeDataWindowExt},
};
use crate::{
    event::{ElementState, MouseButton},
    math::{Point, Points},
    runtime::Runtime,
    scene::SceneTarget,
    style::StyleSheet,
    window::{Event, InputEvent},
    KludgineError, KludgineHandle, KludgineResult,
};
use arena::{HierarchicalArena, Index};
use once_cell::sync::OnceCell;
use std::collections::{HashMap, HashSet};

static UI: OnceCell<HierarchicalArena> = OnceCell::new();

pub(crate) fn global_arena() -> &'static HierarchicalArena {
    UI.get_or_init(HierarchicalArena::default)
}

#[derive(Default, Debug, Clone)]
pub(crate) struct UIState {
    data: KludgineHandle<UIStateData>,
}

impl UIState {
    async fn deactivate(&self) {
        let mut data = self.data.write().await;
        data.active = None;
    }

    async fn activate(&self, index: Index) {
        let mut data = self.data.write().await;
        data.active = Some(index);
    }

    // async fn focus(&self, index: Index) {
    //     let mut data = self.data.write().await;
    //     data.focus = Some(index);
    // }

    // async fn blur(&self) {
    //     let mut data = self.data.write().await;
    //     data.focus = None;
    // }

    async fn focused(&self) -> Option<Index> {
        let data = self.data.read().await;
        data.focus
    }

    async fn active(&self) -> Option<Index> {
        let data = self.data.read().await;
        data.active
    }
}

#[derive(Default, Debug)]
struct UIStateData {
    focus: Option<Index>,
    active: Option<Index>,
}

pub struct UserInterface<C>
where
    C: InteractiveComponent + 'static,
{
    pub(crate) root: Entity<C>,
    ui_state: UIState,
    mouse_button_handlers: HashMap<MouseButton, Index>,
    hover: Option<Index>,
    last_render_order: Vec<Index>,
    last_mouse_position: Option<Point<Points>>,
}

impl<C> UserInterface<C>
where
    C: InteractiveComponent + 'static,
{
    pub async fn new(root: C, scene: SceneTarget) -> KludgineResult<Self> {
        let root = Entity::new({
            let node = Node::new(root, StyleSheet::default(), AbsoluteBounds::default(), None);

            global_arena().insert(None, node).await
        });

        let ui = Self {
            root,
            hover: None,
            last_render_order: Default::default(),
            ui_state: Default::default(),
            last_mouse_position: None,
            mouse_button_handlers: Default::default(),
        };
        ui.initialize(root, scene).await?;
        Ok(ui)
    }

    pub async fn render(&mut self, scene: &SceneTarget) -> KludgineResult<()> {
        let hovered_indicies = self.hovered_indicies().await;
        let layouts = LayoutEngine::layout(
            global_arena(),
            &self.ui_state,
            self.root.into(),
            scene,
            hovered_indicies,
        )
        .await?;

        self.last_render_order.clear();
        while let Some(index) = layouts.next_to_render().await {
            if let Some(layout) = layouts.get_layout(&index).await {
                self.last_render_order.push(index);
                let node = global_arena().get(index).await.unwrap();
                let mut context = StyledContext::new(
                    index,
                    scene.clone(),
                    layouts
                        .effective_styles()
                        .await
                        .get(&index)
                        .unwrap()
                        .clone(),
                    global_arena().clone(),
                    self.ui_state.clone(),
                );
                node.render_background(&mut context, &layout).await?;
                node.render(&mut context, &layout).await?;
            }
        }

        Ok(())
    }

    async fn hovered_indicies(&mut self) -> HashSet<Index> {
        let mut indicies = HashSet::new();
        let mut hovered_index = self.hover;
        while let Some(index) = hovered_index {
            indicies.insert(index);
            hovered_index = global_arena().parent(index).await;
        }
        indicies
    }

    pub async fn update(&mut self, scene: &SceneTarget) -> KludgineResult<()> {
        // Loop twice, once to allow all the pending messages to be exhausted across all
        // nodes. Then after all messages have been processed, trigger the update method
        // for each node.

        let mut traverser = global_arena().traverse(self.root).await;
        while let Some(index) = traverser.next().await {
            let mut context = Context::new(index, global_arena().clone(), self.ui_state.clone());
            let node = global_arena().get(index).await.unwrap();

            node.process_pending_events(&mut context).await?;
        }

        let mut traverser = global_arena().traverse(self.root).await;
        while let Some(index) = traverser.next().await {
            let mut context = SceneContext::new(
                index,
                scene.clone(),
                global_arena().clone(),
                self.ui_state.clone(),
            );
            let node = global_arena().get(index).await.unwrap();

            node.update(&mut context).await?;
        }

        Ok(())
    }

    pub async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        match event.event {
            Event::MouseMoved { position } => {
                self.last_mouse_position = position;

                for (&button, &index) in self.mouse_button_handlers.iter() {
                    if let Some(node) = global_arena().get(index).await {
                        let mut context =
                            Context::new(index, global_arena().clone(), self.ui_state.clone());
                        node.mouse_drag(&mut context, &position, button).await?;
                    }
                }

                self.hover = None;
                if let Some(position) = position {
                    for &index in self.last_render_order.iter() {
                        if let Some(node) = global_arena().get(index).await {
                            let mut context =
                                Context::new(index, global_arena().clone(), self.ui_state.clone());
                            if node.hit_test(&mut context, &position).await? {
                                self.hover = Some(index);
                                break;
                            }
                        }
                    }
                }
            }
            Event::MouseWheel { .. } => {} //{todo!("Hook up mouse scroll to hovered nodes"),
            Event::MouseButton { button, state } => match state {
                ElementState::Released => {
                    if let Some(&index) = self.mouse_button_handlers.get(&button) {
                        if let Some(node) = global_arena().get(index).await {
                            let mut context =
                                Context::new(index, global_arena().clone(), self.ui_state.clone());
                            node.mouse_up(&mut context, &self.last_mouse_position, button)
                                .await?;
                        }
                    }

                    self.mouse_button_handlers.remove(&button);
                }
                ElementState::Pressed => {
                    self.ui_state.deactivate().await;
                    self.mouse_button_handlers.remove(&button);

                    if let Some(last_mouse_position) = self.last_mouse_position {
                        let mut next_to_process = self.hover;
                        while let Some(index) = next_to_process {
                            if let Some(node) = global_arena().get(index).await {
                                let mut context = Context::new(
                                    index,
                                    global_arena().clone(),
                                    self.ui_state.clone(),
                                );
                                if let EventStatus::Handled = node
                                    .mouse_down(&mut context, &last_mouse_position, button)
                                    .await?
                                {
                                    self.mouse_button_handlers.insert(button, index);
                                    break;
                                }
                            }
                            next_to_process = global_arena().parent(index).await;
                        }
                    }
                }
            },
            _ => {}
        }
        Ok(())
    }

    async fn initialize(&self, index: impl Into<Index>, scene: SceneTarget) -> KludgineResult<()> {
        let index = index.into();
        let node = global_arena().get(index).await.unwrap();

        node.initialize(&mut SceneContext::new(
            index,
            scene,
            global_arena().clone(),
            self.ui_state.clone(),
        ))
        .await
    }
}

impl<C> Drop for UserInterface<C>
where
    C: InteractiveComponent + 'static,
{
    fn drop(&mut self) {
        let root = self.root;
        Runtime::spawn(async move {
            global_arena().remove(root).await;
        });
    }
}

#[derive(Debug)]
pub struct Entity<C> {
    index: Option<Index>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C> Default for Entity<C> {
    fn default() -> Self {
        Self {
            index: None,
            _phantom: Default::default(),
        }
    }
}

impl<C> Into<Index> for Entity<C> {
    fn into(self) -> Index {
        self.index.expect("Using uninitialized Entity")
    }
}

impl<C> Entity<C> {
    pub fn new(index: Index) -> Self {
        Self {
            index: Some(index),
            _phantom: Default::default(),
        }
    }
}

impl<C> Entity<C>
where
    C: InteractiveComponent + 'static,
{
    pub async fn send(&self, message: C::Input) -> KludgineResult<()> {
        if let Some(target_node) = global_arena()
            .get(self.index.expect("Using uninitialized Entity"))
            .await
        {
            let component = target_node.component.read().await;
            if let Some(node_data) = component.as_any().downcast_ref::<NodeData<C>>() {
                let _ = node_data.input_sender.send(message);
                Ok(())
            } else {
                unreachable!("Invalid type in Entity<T> -- Node contained different type than T")
            }
        } else {
            Err(KludgineError::InvalidIndex)
        }
    }
}

impl<C> Entity<C>
where
    C: AnimatableComponent + 'static,
{
    pub async fn animate(&self) -> C::AnimationFactory {
        C::new_animation_factory(self.index.expect("Using uninitialized Entity"))
    }
}

impl<C> Clone for Entity<C> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            _phantom: Default::default(),
        }
    }
}

impl<C> Copy for Entity<C> {}
