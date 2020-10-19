mod animation;
mod arena;
mod button;
mod component;
mod context;
mod control;
mod image;
mod label;
mod layout;
#[cfg(feature = "ecs")]
pub mod legion;
mod node;
mod timeout;

pub(crate) use self::node::NodeData;
pub use self::{
    animation::{AnimationManager, LinearTransition},
    button::{Button, ButtonStyle},
    component::{
        AnimatableComponent, Callback, Component, EntityBuilder, InteractiveComponent,
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
    timeout::Timeout,
};
use crate::{
    event::{ElementState, MouseButton},
    math::{Point, Scaled},
    runtime::Runtime,
    scene::Scene,
    style::StyleSheet,
    window::EventStatus,
    window::{Event, InputEvent, WindowEvent},
    Handle, KludgineError, KludgineResult, RequiresInitialization,
};
pub use arena::{HierarchicalArena, Index};
use async_channel::Sender;
use once_cell::sync::OnceCell;
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

static UI: OnceCell<HierarchicalArena> = OnceCell::new();

pub(crate) fn global_arena() -> &'static HierarchicalArena {
    UI.get_or_init(HierarchicalArena::default)
}

#[derive(Debug, Clone)]
pub(crate) struct UIState {
    data: Handle<UIStateData>,
}

impl UIState {
    pub(crate) fn new(event_sender: Sender<WindowEvent>) -> Self {
        Self {
            data: Handle::new(UIStateData::new(event_sender)),
        }
    }

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
        if !data.needs_render {
            data.needs_render = true;
            let _ = data.event_sender.send(WindowEvent::WakeUp).await;
        }
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
    event_sender: Sender<WindowEvent>,
}

impl UIStateData {
    fn new(event_sender: Sender<WindowEvent>) -> Self {
        Self {
            needs_render: true,
            focus: None,
            active: None,
            next_redraw_target: RedrawTarget::default(),
            event_sender,
        }
    }
}

pub struct UserInterface<C>
where
    C: InteractiveComponent + 'static,
{
    pub(crate) root: Entity<C>,
    arena: HierarchicalArena,
    ui_state: UIState,
    mouse_button_handlers: HashMap<MouseButton, Index>,
    hover: Option<Index>,
    last_render_order: Vec<Index>,
    last_mouse_position: Option<Point<f32, Scaled>>,
}

impl<C> UserInterface<C>
where
    C: InteractiveComponent + 'static,
{
    pub(crate) async fn new(
        root: C,
        scene: Scene,
        arena: HierarchicalArena,
        event_sender: Sender<WindowEvent>,
    ) -> KludgineResult<Self> {
        let ui_state = UIState::new(event_sender);
        let root = Entity::new(Context::new(
            {
                let node = Node::new::<C>(
                    root,
                    StyleSheet::default(),
                    AbsoluteBounds::default(),
                    true,
                    None,
                );

                arena.insert(None, node).await
            },
            arena.clone(),
            ui_state.clone(),
        ));

        let ui = Self {
            root: root.clone(),
            arena,
            hover: None,
            last_render_order: Default::default(),
            ui_state,
            last_mouse_position: None,
            mouse_button_handlers: Default::default(),
        };
        ui.initialize(&root, scene).await?;
        Ok(ui)
    }

    pub async fn render(&mut self, scene: &Scene) -> KludgineResult<()> {
        let hovered_indicies = self.hovered_indicies().await;
        let layouts = LayoutEngine::layout(
            &self.arena,
            &self.ui_state,
            self.root.index(),
            scene,
            hovered_indicies,
        )
        .await?;

        self.last_render_order.clear();
        while let Some(index) = layouts.next_to_render().await {
            if let Some(layout) = layouts.get_layout(&index).await {
                self.last_render_order.push(index);
                let node = self.arena.get(&index).await.unwrap();
                let mut context = StyledContext::new(
                    index,
                    scene.clone(),
                    layouts.effective_styles().await.clone(),
                    self.arena.clone(),
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
            hovered_index = self.arena.parent(index).await;
        }
        indicies
    }

    pub async fn update(&mut self, scene: &Scene, target_fps: Option<u16>) -> KludgineResult<()> {
        // Loop twice, once to allow all the pending messages to be exhausted across all
        // nodes. Then after all messages have been processed, trigger the update method
        // for each node.
        self.ui_state.initialize_redraw_target(target_fps).await;

        let mut traverser = self.arena.traverse(&self.root).await;
        while let Some(index) = traverser.next().await {
            let mut context = SceneContext::new(
                index,
                scene.clone(),
                self.arena.clone(),
                self.ui_state.clone(),
            );

            if let Some(node) = self.arena.get(&index).await {
                node.update(&mut context).await?;
            }
        }

        Ok(())
    }

    pub async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        match event.event {
            Event::MouseMoved { position } => {
                self.last_mouse_position = position;

                for (&button, &index) in self.mouse_button_handlers.iter() {
                    if let Some(node) = self.arena.get(&index).await {
                        let mut context =
                            Context::new(index, self.arena.clone(), self.ui_state.clone());
                        node.mouse_drag(&mut context, position, button).await?;
                    }
                }

                let starting_hovered_indicies = self.hovered_indicies().await;
                self.hover = None;
                if let Some(position) = position {
                    for &index in self.last_render_order.iter() {
                        if let Some(node) = self.arena.get(&index).await {
                            let mut context =
                                Context::new(index, self.arena.clone(), self.ui_state.clone());
                            if node.hit_test(&mut context, position).await? {
                                self.hover = Some(index);
                                break;
                            }
                        }
                    }
                }
                let current_hovered_indicies = self.hovered_indicies().await;

                for &new_hover in current_hovered_indicies.difference(&starting_hovered_indicies) {
                    if let Some(node) = self.arena.get(&new_hover).await {
                        let mut context =
                            Context::new(new_hover, self.arena.clone(), self.ui_state.clone());
                        node.hovered(&mut context).await?;
                    }
                }

                for &new_hover in starting_hovered_indicies.difference(&current_hovered_indicies) {
                    if let Some(node) = self.arena.get(&new_hover).await {
                        let mut context =
                            Context::new(new_hover, self.arena.clone(), self.ui_state.clone());
                        node.unhovered(&mut context).await?;
                    }
                }

                if current_hovered_indicies != starting_hovered_indicies {
                    self.ui_state.set_needs_redraw().await;
                }
            }
            Event::MouseWheel { delta, touch_phase } => {
                let mut next_to_process = self.hover;
                while let Some(index) = next_to_process {
                    if let Some(node) = self.arena.get(&index).await {
                        let mut context =
                            Context::new(index, self.arena.clone(), self.ui_state.clone());
                        if node.interactive().await {
                            if let EventStatus::Processed =
                                node.mouse_wheel(&mut context, delta, touch_phase).await?
                            {
                                break;
                            }
                        }
                    }
                    next_to_process = self.arena.parent(index).await;
                }
            }
            Event::MouseButton { button, state } => match state {
                ElementState::Released => {
                    if let Some(&index) = self.mouse_button_handlers.get(&button) {
                        if let Some(node) = self.arena.get(&index).await {
                            let mut context =
                                Context::new(index, self.arena.clone(), self.ui_state.clone());
                            node.mouse_up(&mut context, self.last_mouse_position, button)
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
                            if let Some(node) = self.arena.get(&index).await {
                                let mut context =
                                    Context::new(index, self.arena.clone(), self.ui_state.clone());
                                if node.interactive().await {
                                    if let EventStatus::Processed = node
                                        .mouse_down(&mut context, last_mouse_position, button)
                                        .await?
                                    {
                                        self.mouse_button_handlers.insert(button, index);
                                        break;
                                    }
                                }
                            }
                            next_to_process = self.arena.parent(index).await;
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

    async fn initialize(&self, index: &impl Indexable, scene: Scene) -> KludgineResult<()> {
        let index = index.index();
        let node = self.arena.get(&index).await.unwrap();

        node.initialize(&mut SceneContext::new(
            index,
            scene,
            self.arena.clone(),
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
        let root = std::mem::take(&mut self.root);
        let arena = self.arena.clone();
        Runtime::spawn(async move {
            arena.remove(&root).await;
        })
        .detach();
    }
}

#[derive(Debug)]
pub struct Entity<C> {
    context: RequiresInitialization<Context>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C> Default for Entity<C> {
    fn default() -> Self {
        Self {
            context: Default::default(),
            _phantom: Default::default(),
        }
    }
}

pub trait Indexable {
    fn index(&self) -> Index;
}

impl<C> Indexable for Entity<C> {
    fn index(&self) -> Index {
        self.context.index()
    }
}

impl Indexable for Index {
    fn index(&self) -> Index {
        *self
    }
}

impl<C> Entity<C> {
    pub fn new(context: Context) -> Self {
        Self {
            context: RequiresInitialization::new(context),
            _phantom: Default::default(),
        }
    }
}

impl<C> Entity<C>
where
    C: InteractiveComponent + 'static,
{
    pub async fn send(&self, command: C::Command) -> KludgineResult<()> {
        if let Some(target_node) = global_arena().get(self).await {
            let component = target_node.component.read().await;
            if let Some(component_handle) = component.component::<C, C>().await {
                let mut context = self.context.clone();
                Runtime::spawn(async move {
                    let mut component = component_handle.write().await;
                    component
                        .receive_command(&mut context, command)
                        .await
                        .unwrap()
                })
                .detach();

                Ok(())
            } else {
                unreachable!("Invalid type in Entity<T> -- Node contained different type than T")
            }
        } else {
            Err(KludgineError::InvalidIndex)
        }
    }

    pub async fn component<T: Send + Sync + 'static>(&self) -> Option<Handle<T>> {
        if let Some(target_node) = global_arena().get(self).await {
            let component = target_node.component.read().await;
            component.component::<C, T>().await
        } else {
            None
        }
    }
}

impl<C> Entity<C>
where
    C: AnimatableComponent + 'static,
{
    pub fn animate(&self) -> C::AnimationFactory {
        C::new_animation_factory(self.clone())
    }
}

impl<C> Clone for Entity<C> {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
            _phantom: Default::default(),
        }
    }
}
