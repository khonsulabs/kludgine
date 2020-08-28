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
    animation::{AnimationManager, LinearTransition},
    button::{Button, ButtonStyle},
    component::{
        AnimatableComponent, Callback, Component, EntityBuilder, EventStatus, InteractiveComponent,
        LayoutConstraints, StandaloneComponent,
    },
    context::*,
    control::ControlEvent,
    image::{
        Image, ImageAlphaAnimation, ImageCommand, ImageFrameAnimation, ImageOptions, ImageScaling,
    },
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
    KludgineError, KludgineHandle, KludgineResult, RequiresInitialization,
};
pub use arena::{HierarchicalArena, Index};
use once_cell::sync::OnceCell;
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

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
        if data.active != None {
            data.needs_render = true;
            data.active = None;
        }
    }

    async fn activate(&self, index: Index) {
        let mut data = self.data.write().await;
        if data.active != Some(index) {
            data.needs_render = true;
            data.active = Some(index);
        }
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

    async fn set_needs_redraw(&self) {
        let mut data = self.data.write().await;
        data.needs_render = true;
    }

    async fn clear_redraw_target(&self) {
        let mut data = self.data.write().await;
        data.needs_render = false;
        data.next_redraw_target = RedrawTarget::None;
    }

    async fn initialize_redraw_target(&self, target_fps: Option<u16>) {
        let mut data = self.data.write().await;
        if let RedrawTarget::None = data.next_redraw_target {
            match target_fps {
                Some(fps) => {
                    data.next_redraw_target = RedrawTarget::Scheduled(
                        Instant::now()
                            .checked_add(Duration::from_secs_f32(1. / fps as f32))
                            .unwrap(),
                    );
                }
                None => {
                    data.next_redraw_target = RedrawTarget::Never;
                }
            }
        }
    }

    async fn estimate_next_frame(&self, duration: Duration) {
        self.estimate_next_frame_instant(Instant::now().checked_add(duration).unwrap())
            .await;
    }

    async fn estimate_next_frame_instant(&self, instant: Instant) {
        let mut data = self.data.write().await;
        match data.next_redraw_target {
            RedrawTarget::Never | RedrawTarget::None => {
                data.next_redraw_target = RedrawTarget::Scheduled(instant);
            }
            RedrawTarget::Scheduled(existing_instant) => {
                if instant < existing_instant {
                    data.next_redraw_target = RedrawTarget::Scheduled(instant);
                }
            }
        }
    }

    async fn next_redraw_target(&self) -> RedrawTarget {
        let data = self.data.read().await;
        data.next_redraw_target
    }

    async fn needs_render(&self) -> bool {
        let data = self.data.read().await;
        data.needs_render
            || match data.next_redraw_target {
                RedrawTarget::Never => false,
                RedrawTarget::None => false,
                RedrawTarget::Scheduled(scheduled_for) => scheduled_for < Instant::now(),
            }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RedrawTarget {
    None,
    Never,
    Scheduled(Instant),
}

impl Default for RedrawTarget {
    fn default() -> Self {
        Self::None
    }
}

pub(crate) enum UpdateSchedule {
    Now,
    Scheduled(Instant),
}

impl RedrawTarget {
    pub fn next_update_instant(&self) -> Option<UpdateSchedule> {
        match self {
            RedrawTarget::Never => None,
            Self::None => Some(UpdateSchedule::Now),
            Self::Scheduled(scheduled_for) => Some(UpdateSchedule::Scheduled(*scheduled_for)),
        }
    }
}

impl UpdateSchedule {
    pub fn timeout_target(&self) -> Option<Instant> {
        match self {
            UpdateSchedule::Now => None,
            UpdateSchedule::Scheduled(scheduled_for) => {
                if &Instant::now() > scheduled_for {
                    None
                } else {
                    Some(*scheduled_for)
                }
            }
        }
    }
}

#[derive(Debug)]
struct UIStateData {
    focus: Option<Index>,
    active: Option<Index>,
    next_redraw_target: RedrawTarget,
    needs_render: bool,
}

impl Default for UIStateData {
    fn default() -> Self {
        Self {
            needs_render: true,
            focus: None,
            active: None,
            next_redraw_target: RedrawTarget::default(),
        }
    }
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
            let node = Node::new(
                root,
                StyleSheet::default(),
                AbsoluteBounds::default(),
                true,
                None,
            );

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

        self.last_render_order.reverse();

        self.ui_state.clear_redraw_target().await;

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

    pub async fn update(
        &mut self,
        scene: &SceneTarget,
        target_fps: Option<u16>,
    ) -> KludgineResult<()> {
        // Loop twice, once to allow all the pending messages to be exhausted across all
        // nodes. Then after all messages have been processed, trigger the update method
        // for each node.
        self.ui_state.initialize_redraw_target(target_fps).await;

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

                let starting_hovered_indicies = self.hovered_indicies().await;
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
                let current_hovered_indicies = self.hovered_indicies().await;

                for &new_hover in current_hovered_indicies.difference(&starting_hovered_indicies) {
                    if let Some(node) = global_arena().get(new_hover).await {
                        let mut context =
                            Context::new(new_hover, global_arena().clone(), self.ui_state.clone());
                        node.hovered(&mut context).await?;
                    }
                }

                for &new_hover in starting_hovered_indicies.difference(&current_hovered_indicies) {
                    if let Some(node) = global_arena().get(new_hover).await {
                        let mut context =
                            Context::new(new_hover, global_arena().clone(), self.ui_state.clone());
                        node.unhovered(&mut context).await?;
                    }
                }

                if current_hovered_indicies != starting_hovered_indicies {
                    self.ui_state.set_needs_redraw().await;
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
                                if node.interactive().await {
                                    if let EventStatus::Handled = node
                                        .mouse_down(&mut context, &last_mouse_position, button)
                                        .await?
                                    {
                                        self.mouse_button_handlers.insert(button, index);
                                        break;
                                    }
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

    pub async fn request_redraw(&self) {
        self.ui_state.set_needs_redraw().await;
    }

    pub(crate) async fn next_redraw_target(&self) -> RedrawTarget {
        self.ui_state.next_redraw_target().await
    }

    pub(crate) async fn needs_render(&self) -> bool {
        self.ui_state.needs_render().await
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
        })
        .detach();
    }
}

#[derive(Debug)]
pub struct Entity<C> {
    index: RequiresInitialization<Index>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C> Default for Entity<C> {
    fn default() -> Self {
        Self {
            index: Default::default(),
            _phantom: Default::default(),
        }
    }
}

impl<C> Into<Index> for Entity<C> {
    fn into(self) -> Index {
        *self.index
    }
}

impl<C> Entity<C> {
    pub fn new(index: Index) -> Self {
        Self {
            index: index.into(),
            _phantom: Default::default(),
        }
    }

    pub fn index(&self) -> Index {
        *self.index
    }
}

impl<C> Entity<C>
where
    C: InteractiveComponent + 'static,
{
    pub async fn send(&self, message: C::Input) -> KludgineResult<()> {
        if let Some(target_node) = global_arena().get(*self.index).await {
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
    pub fn animate(&self) -> C::AnimationFactory {
        C::new_animation_factory(*self)
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
