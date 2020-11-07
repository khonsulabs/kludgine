mod animation;
mod arena;
mod component;
mod context;
mod layout;
mod node;
mod timeout;

use self::node::ThreadsafeAnyMap;
pub use self::{
    animation::{AnimationManager, LinearTransition},
    component::*,
    context::*,
    layout::*,
    node::Node,
    timeout::Timeout,
};
use crate::{
    math::{Point, Scaled},
    runtime::Runtime,
    scene::{Scene, Target},
    style::theme::{Classes, Id},
    window::event::{ElementState, Event, EventStatus, InputEvent, MouseButton, WindowEvent},
    Handle, KludgineError, KludgineResult, RequiresInitialization,
};
pub use arena::{HierarchicalArena, Index};
use async_channel::Sender;
use once_cell::sync::OnceCell;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
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

    async fn removed_element(&self, index: Index) {
        let mut data = self.data.write().await;
        if let Some(index) = data
            .layers
            .iter()
            .enumerate()
            .find(|(_, layer)| layer.root == index)
            .map(|(index, _)| index)
        {
            data.layers.remove(index);
        }
    }

    async fn top_layer(&self) -> UILayer {
        let data = self.data.read().await;
        data.layers.last().cloned().unwrap()
    }

    async fn layers(&self) -> Vec<UILayer> {
        let data = self.data.read().await;
        data.layers.clone()
    }

    async fn clear_layer_states(&self) {
        let data = self.data.read().await;
        futures::future::join_all(data.layers.iter().map(|l| l.clear_state())).await;
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

    pub(crate) async fn push_layer_from_index(
        &self,
        root: Index,
        arena: &HierarchicalArena,
        scene: &Scene,
    ) -> KludgineResult<()> {
        let layer = {
            let mut data = self.data.write().await;

            let layer = UILayer {
                root,
                data: Handle::new(UILayerData::default()),
            };
            data.layers.push(layer.clone());
            layer
        };

        let node = arena.get(&root).await.unwrap();

        node.initialize(&mut Context::new(
            LayerIndex { layer, index: root },
            arena.clone(),
            self.clone(),
            Target::from(scene.clone()),
        ))
        .await?;

        Ok(())
    }

    async fn push_layer<C: InteractiveComponent + 'static>(
        &self,
        root: C,
        arena: &HierarchicalArena,
        scene: &Scene,
    ) -> KludgineResult<Index> {
        let mut components = ThreadsafeAnyMap::new();
        let theme = scene.theme().await;
        components.insert(Id::from("root"));
        if let Some(classes) = root.classes() {
            components.insert(Classes(classes));
        }

        let stylesheet = theme.stylesheet_for(components.get(), components.get());
        components.insert(Handle::new(stylesheet));

        components.insert(Handle::new(root));
        components.insert(Handle::new(AbsoluteBounds::default()));

        let root = arena
            .insert(None, Node::from_components::<C>(components, true, None))
            .await;

        self.push_layer_from_index(root, arena, scene).await?;
        Ok(root)
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
    layers: Vec<UILayer>,
    next_redraw_target: RedrawTarget,
    needs_render: bool,
    event_sender: Sender<WindowEvent>,
}

#[derive(Debug, Clone)]
pub struct UILayer {
    pub root: Index,
    data: Handle<UILayerData>,
}

impl UILayer {
    async fn clear_state(&self) {
        let mut data = self.data.write().await;
        data.active = None;
        data.focus = None;
        println!("Cleared state");
    }

    async fn activate(&self, index: Index, state: &UIState) {
        let mut data = self.data.write().await;
        if data.active != Some(index) {
            data.active = Some(index);
            state.set_needs_redraw().await;
        }
    }

    async fn deactivate(&self, state: &UIState) {
        let mut data = self.data.write().await;
        if data.active != None {
            data.active = None;
            state.set_needs_redraw().await;
        }
    }

    async fn active(&self) -> Option<Index> {
        let data = self.data.read().await;
        data.active
    }

    async fn focus(&self) -> Option<Index> {
        let data = self.data.read().await;
        data.focus
    }

    async fn focus_on(&self, focus: Option<Index>, state: &UIState) {
        let mut data = self.data.write().await;
        if data.focus != focus {
            data.focus = focus;
            state.set_needs_redraw().await;
        }
    }
}

#[derive(Default, Debug, Clone)]
struct UILayerData {
    focus: Option<Index>,
    active: Option<Index>,
}

impl UIStateData {
    fn new(event_sender: Sender<WindowEvent>) -> Self {
        Self {
            event_sender,
            layers: Default::default(),
            needs_render: true,
            next_redraw_target: RedrawTarget::default(),
        }
    }
}

pub struct UserInterface<C>
where
    C: InteractiveComponent + 'static,
{
    arena: HierarchicalArena,
    ui_state: UIState,
    mouse_button_handlers: HashMap<MouseButton, LayerIndex>,
    hover: Option<LayerIndex>,
    last_render_order: Vec<LayerIndex>,
    last_mouse_position: Option<Point<f32, Scaled>>,
    scene: Scene,
    _phantom: std::marker::PhantomData<C>,
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

        let ui = Self {
            scene,
            arena,
            hover: None,
            last_render_order: Default::default(),
            ui_state,
            last_mouse_position: None,
            mouse_button_handlers: Default::default(),
            _phantom: Default::default(),
        };
        ui.ui_state.push_layer(root, &ui.arena, &ui.scene).await?;
        Ok(ui)
    }

    pub(crate) async fn layers(&self) -> Vec<UILayer> {
        self.ui_state.layers().await
    }

    pub async fn render(&mut self) -> KludgineResult<()> {
        let scene_scale = self.scene.scale_factor().await;
        self.last_render_order.clear();

        for layer in self.ui_state.layers().await {
            let hovered_indicies = self.hovered_indicies().await;
            let layouts = LayoutEngine::layout(
                &self.arena,
                &layer,
                &self.ui_state,
                layer.root,
                &Target::from(self.scene.clone()),
                hovered_indicies
                    .into_iter()
                    .filter(|li| Handle::ptr_eq(&li.layer.data, &layer.data))
                    .map(|li| li.index)
                    .collect(),
            )
            .await?;

            while let Some(index) = layouts.next_to_render().await {
                if let Some(layout) = layouts.get_layout(&index).await {
                    let layer_index = LayerIndex {
                        layer: layer.clone(),
                        index,
                    };
                    self.last_render_order.push(layer_index.clone());
                    if let Some(node) = self.arena.get(&index).await {
                        let mut context = StyledContext::new(
                            layer_index,
                            Target::from(self.scene.clone())
                                .clipped_to((layout.inner_bounds() * scene_scale).round().to_u32()),
                            layouts.effective_styles().await.clone(),
                            self.arena.clone(),
                            self.ui_state.clone(),
                        );
                        node.render_background(&mut context, &layout).await?;
                        node.render(&mut context, &layout).await?;
                    }
                }
            }
        }

        self.last_render_order.reverse();
        self.ui_state.clear_redraw_target().await;

        Ok(())
    }

    async fn hovered_indicies(&mut self) -> HashSet<LayerIndex> {
        let mut indicies = HashSet::new();
        let mut hovered_index = self.hover.clone();
        while let Some(layer_index) = hovered_index {
            hovered_index = self
                .arena
                .parent(layer_index.index)
                .await
                .map(|index| LayerIndex {
                    index,
                    layer: layer_index.layer.clone(),
                });
            indicies.insert(layer_index);
        }
        indicies
    }

    pub async fn update(&mut self, scene: &Target, target_fps: Option<u16>) -> KludgineResult<()> {
        // Loop twice, once to allow all the pending messages to be exhausted across all
        // nodes. Then after all messages have been processed, trigger the update method
        // for each node.
        self.ui_state.initialize_redraw_target(target_fps).await;

        for layer in self.ui_state.layers().await {
            let mut traverser = self.arena.traverse(&layer.root).await;
            while let Some(index) = traverser.next().await {
                let mut context = Context::new(
                    LayerIndex {
                        index,
                        layer: layer.clone(),
                    },
                    self.arena.clone(),
                    self.ui_state.clone(),
                    scene.clone(),
                );

                if let Some(node) = self.arena.get(&index).await {
                    node.update(&mut context).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn receive_character(&mut self, character: char) -> KludgineResult<()> {
        let top_layer = self.ui_state.top_layer().await;
        let event_target = top_layer.focus().await.unwrap_or(top_layer.root);
        if let Some(node) = self.arena.get(&event_target).await {
            let mut context = Context::new(
                LayerIndex {
                    index: event_target,
                    layer: top_layer,
                },
                self.arena.clone(),
                self.ui_state.clone(),
                Target::from(self.scene.clone()),
            );
            node.receive_character(&mut context, character).await?
        }
        Ok(())
    }

    pub async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        match event.event {
            Event::MouseMoved { position } => {
                self.last_mouse_position = position;

                for (&button, layer_index) in self.mouse_button_handlers.iter() {
                    if let Some(node) = self.arena.get(&layer_index.index).await {
                        let mut context = Context::new(
                            layer_index.clone(),
                            self.arena.clone(),
                            self.ui_state.clone(),
                            Target::from(self.scene.clone()),
                        );
                        node.mouse_drag(&mut context, position, button).await?;
                    }
                }

                let starting_hovered_indicies = self.hovered_indicies().await;
                self.hover = None;
                if let Some(position) = position {
                    for layer_index in self.last_render_order.iter() {
                        if let Some(node) = self.arena.get(&layer_index.index).await {
                            let mut context = Context::new(
                                layer_index.clone(),
                                self.arena.clone(),
                                self.ui_state.clone(),
                                Target::from(self.scene.clone()),
                            );
                            if node.hit_test(&mut context, position).await? {
                                self.hover = Some(layer_index.clone());
                                break;
                            }
                        }
                    }
                }
                let current_hovered_indicies = self.hovered_indicies().await;

                for new_hover in current_hovered_indicies.difference(&starting_hovered_indicies) {
                    if let Some(node) = self.arena.get(&new_hover.index).await {
                        let mut context = Context::new(
                            new_hover.clone(),
                            self.arena.clone(),
                            self.ui_state.clone(),
                            Target::from(self.scene.clone()),
                        );
                        node.hovered(&mut context).await?;
                    }
                }

                for new_hover in starting_hovered_indicies.difference(&current_hovered_indicies) {
                    if let Some(node) = self.arena.get(&new_hover.index).await {
                        let mut context = Context::new(
                            new_hover.clone(),
                            self.arena.clone(),
                            self.ui_state.clone(),
                            Target::from(self.scene.clone()),
                        );
                        node.unhovered(&mut context).await?;
                    }
                }

                if current_hovered_indicies != starting_hovered_indicies {
                    self.ui_state.set_needs_redraw().await;
                }
            }
            Event::MouseWheel { delta, touch_phase } => {
                let mut next_to_process = self.hover.clone();
                while let Some(layer_index) = next_to_process {
                    if let Some(node) = self.arena.get(&layer_index.index).await {
                        let mut context = Context::new(
                            layer_index.clone(),
                            self.arena.clone(),
                            self.ui_state.clone(),
                            Target::from(self.scene.clone()),
                        );
                        if node.interactive().await {
                            if let EventStatus::Processed =
                                node.mouse_wheel(&mut context, delta, touch_phase).await?
                            {
                                break;
                            }
                        }
                    }
                    next_to_process =
                        self.arena
                            .parent(layer_index.index)
                            .await
                            .map(|index| LayerIndex {
                                index,
                                layer: layer_index.layer.clone(),
                            });
                }
            }
            Event::MouseButton { button, state } => match state {
                ElementState::Released => {
                    if let Some(layer_index) = self.mouse_button_handlers.get(&button) {
                        if let Some(node) = self.arena.get(&layer_index.index).await {
                            let mut context = Context::new(
                                layer_index.clone(),
                                self.arena.clone(),
                                self.ui_state.clone(),
                                Target::from(self.scene.clone()),
                            );
                            node.mouse_up(&mut context, self.last_mouse_position, button)
                                .await?;
                        }
                    }

                    self.mouse_button_handlers.remove(&button);
                }
                ElementState::Pressed => {
                    self.ui_state.clear_layer_states().await;
                    self.mouse_button_handlers.remove(&button);

                    if let Some(last_mouse_position) = self.last_mouse_position {
                        let mut next_to_process = self.hover.clone();
                        while let Some(layer_index) = next_to_process {
                            if let Some(node) = self.arena.get(&layer_index.index).await {
                                let mut context = Context::new(
                                    layer_index.clone(),
                                    self.arena.clone(),
                                    self.ui_state.clone(),
                                    Target::from(self.scene.clone()),
                                );
                                if node.interactive().await {
                                    if let EventStatus::Processed = node
                                        .mouse_down(&mut context, last_mouse_position, button)
                                        .await?
                                    {
                                        self.mouse_button_handlers
                                            .insert(button, layer_index.clone());
                                        break;
                                    }
                                }
                            }
                            next_to_process =
                                self.arena.parent(layer_index.index).await.map(|index| {
                                    LayerIndex {
                                        index,
                                        layer: layer_index.layer.clone(),
                                    }
                                });
                        }
                    }
                }
            },
            Event::Keyboard {
                key,
                state,
                scancode,
            } => {
                let top_layer = self.ui_state.top_layer().await;
                let event_target = top_layer.focus().await.unwrap_or(top_layer.root);
                if let Some(node) = self.arena.get(&event_target).await {
                    let mut context = Context::new(
                        LayerIndex {
                            index: event_target,
                            layer: top_layer,
                        },
                        self.arena.clone(),
                        self.ui_state.clone(),
                        Target::from(self.scene.clone()),
                    );
                    node.keyboard_event(&mut context, scancode, key, state)
                        .await?
                }
            }
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
}

impl<C> Drop for UserInterface<C>
where
    C: InteractiveComponent + 'static,
{
    fn drop(&mut self) {
        let ui_state = self.ui_state.clone();
        let arena = self.arena.clone();
        Runtime::spawn(async move {
            for layer in ui_state.layers().await {
                arena.remove(&layer.root).await;
            }
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

pub trait LayerIndexable {
    fn layer_index(&self) -> LayerIndex;
}

impl<C> Indexable for Entity<C> {
    fn index(&self) -> Index {
        self.context.index()
    }
}

impl<C> LayerIndexable for Entity<C> {
    fn layer_index(&self) -> LayerIndex {
        self.context.layer_index()
    }
}

impl Indexable for Index {
    fn index(&self) -> Index {
        *self
    }
}

impl Indexable for LayerIndex {
    fn index(&self) -> Index {
        self.index
    }
}

impl LayerIndexable for LayerIndex {
    fn layer_index(&self) -> LayerIndex {
        self.clone()
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

    pub async fn remove_from_parent(&self) {
        global_arena().remove(self).await;
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

#[derive(Clone, Debug)]
pub struct LayerIndex {
    pub layer: UILayer,
    pub index: Index,
}

impl PartialEq for LayerIndex {
    fn eq(&self, other: &LayerIndex) -> bool {
        self.index.eq(&other.index)
    }
}

impl Eq for LayerIndex {}

impl Hash for LayerIndex {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state)
    }
}
