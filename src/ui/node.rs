use crate::{
    event::{MouseButton, MouseScrollDelta, TouchPhase},
    math::{Point, Scaled, Size},
    runtime::Runtime,
    style::StyleSheet,
    ui::{
        AbsoluteBounds, Callback, Context, EventStatus, InteractiveComponent, Layout, LayoutSolver,
        SceneContext, StyledContext,
    },
    window::{CloseResponse, Window},
    Handle, KludgineResult,
};
use async_trait::async_trait;
use derivative::Derivative;
use std::any::Any;

pub(crate) type ThreadsafeAnyMap = anymap::Map<dyn anymap::any::Any + Send + Sync>;

#[async_trait]
pub(crate) trait AnyNode: CallbackSender + std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn interactive(&self) -> bool;
    async fn style_sheet(&self) -> StyleSheet;
    async fn set_style_sheet(&self, sheet: StyleSheet);
    async fn bounds(&self) -> AbsoluteBounds;
    async fn set_bounds(&self, bounds: AbsoluteBounds);
    async fn set_layout(&self, layout: Layout);
    async fn get_layout(&self) -> Layout;
    async fn receive_message(&self, context: &Context, message: Box<dyn Any + Send + Sync>);

    // Component methods without mutable self

    async fn initialize(&self, context: &mut SceneContext) -> KludgineResult<()>;
    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>>;

    async fn layout(&self, context: &mut StyledContext) -> KludgineResult<Box<dyn LayoutSolver>>;

    async fn render(&self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()>;

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()>;

    async fn update(&self, context: &mut SceneContext) -> KludgineResult<()>;

    async fn mouse_down(
        &self,
        context: &mut Context,
        position: Point<f32, Scaled>,
        button: MouseButton,
    ) -> KludgineResult<EventStatus>;

    async fn mouse_drag(
        &self,
        context: &mut Context,
        position: Option<Point<f32, Scaled>>,
        button: MouseButton,
    ) -> KludgineResult<()>;

    async fn mouse_up(
        &self,
        context: &mut Context,
        position: Option<Point<f32, Scaled>>,
        button: MouseButton,
    ) -> KludgineResult<()>;

    async fn mouse_wheel(
        &self,
        context: &mut Context,
        delta: MouseScrollDelta,
        touch_phase: TouchPhase,
    ) -> KludgineResult<EventStatus>;

    async fn hovered(&self, context: &mut Context) -> KludgineResult<()>;

    async fn unhovered(&self, context: &mut Context) -> KludgineResult<()>;

    async fn hit_test(
        &self,
        context: &mut Context,
        window_position: Point<f32, Scaled>,
    ) -> KludgineResult<bool>;
}

impl dyn AnyNode {
    pub async fn component<C: InteractiveComponent + 'static, T: Send + Sync + 'static>(
        &self,
    ) -> Option<Handle<T>> {
        let components = {
            if let Some(node_data) = self.as_any().downcast_ref::<NodeData<C>>() {
                node_data.components.clone()
            } else {
                return None;
            }
        };
        let components = components.read().await;
        components.get::<Handle<T>>().cloned()
    }
}

#[async_trait]
pub(crate) trait CallbackSender {
    async fn send_callback(&self, input: Box<dyn Any + Send + Sync>);
}

#[async_trait]
impl<T: InteractiveComponent + 'static> AnyNode for NodeData<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn interactive(&self) -> bool {
        self.interactive
    }

    async fn style_sheet(&self) -> StyleSheet {
        let style_sheet = self.style_sheet.read().await;
        style_sheet.clone()
    }

    async fn set_style_sheet(&self, sheet: StyleSheet) {
        let mut style_sheet = self.style_sheet.write().await;
        *style_sheet = sheet;
    }

    async fn bounds(&self) -> AbsoluteBounds {
        let bounds = self.bounds.read().await;
        bounds.clone()
    }

    async fn set_bounds(&self, new_bounds: AbsoluteBounds) {
        let mut bounds = self.bounds.write().await;
        *bounds = new_bounds;
    }

    async fn set_layout(&self, layout: Layout) {
        let mut handle = self.layout.write().await;
        *handle = layout;
    }

    async fn get_layout(&self) -> Layout {
        let layout = self.layout.read().await;
        layout.clone()
    }

    async fn receive_message(&self, context: &Context, message: Box<dyn Any + Send + Sync>) {
        let message = message.downcast_ref::<T::Message>().unwrap().clone();
        let component_handle = self.component::<T>().await.unwrap();
        let mut context = context.clone();
        Runtime::spawn(async move {
            let mut component = component_handle.write().await;
            let _ = component.receive_message(&mut context, message).await;
        })
        .detach();
    }

    async fn initialize(&self, context: &mut SceneContext) -> KludgineResult<()> {
        let component = self.interactive_component().await;
        let mut component = component.write().await;
        component.initialize(context).await
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let component = self.interactive_component().await;
        let component = component.read().await;
        component.content_size(context, constraints).await
    }

    async fn layout(&self, context: &mut StyledContext) -> KludgineResult<Box<dyn LayoutSolver>> {
        let component = self.interactive_component().await;
        let mut component = component.write().await;
        component.layout(context).await
    }

    async fn render(&self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        let component = self.interactive_component().await;
        let component = component.read().await;
        component.render(context, layout).await
    }

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        let component = self.interactive_component().await;
        let component = component.read().await;
        component.render_background(context, layout).await
    }

    async fn update(&self, context: &mut SceneContext) -> KludgineResult<()> {
        let component = self.interactive_component().await;
        let mut component = component.write().await;
        component.update(context).await
    }

    async fn mouse_down(
        &self,
        context: &mut Context,
        position: Point<f32, Scaled>,
        button: MouseButton,
    ) -> KludgineResult<EventStatus> {
        let component = self.interactive_component().await;
        let mut component = component.write().await;
        component.mouse_down(context, position, button).await
    }

    async fn mouse_drag(
        &self,
        context: &mut Context,
        position: Option<Point<f32, Scaled>>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        let component = self.interactive_component().await;
        let mut component = component.write().await;
        component.mouse_drag(context, position, button).await
    }

    async fn mouse_up(
        &self,
        context: &mut Context,
        position: Option<Point<f32, Scaled>>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        let component = self.interactive_component().await;
        let mut component = component.write().await;
        component.mouse_up(context, position, button).await
    }

    async fn mouse_wheel(
        &self,
        context: &mut Context,
        delta: MouseScrollDelta,
        touch_phase: TouchPhase,
    ) -> KludgineResult<EventStatus> {
        let component = self.interactive_component().await;
        let mut component = component.write().await;
        component.mouse_wheel(context, delta, touch_phase).await
    }

    async fn hit_test(
        &self,
        context: &mut Context,
        window_position: Point<f32, Scaled>,
    ) -> KludgineResult<bool> {
        let component = self.interactive_component().await;
        let component = component.read().await;
        component.hit_test(context, window_position).await
    }

    async fn hovered(&self, context: &mut Context) -> KludgineResult<()> {
        let component = self.interactive_component().await;
        let mut component = component.write().await;
        component.hovered(context).await
    }

    async fn unhovered(&self, context: &mut Context) -> KludgineResult<()> {
        let component = self.interactive_component().await;
        let mut component = component.write().await;
        component.unhovered(context).await
    }
}

#[async_trait]
impl<T: InteractiveComponent + 'static> CallbackSender for NodeData<T> {
    async fn send_callback(&self, output: Box<dyn Any + Send + Sync>) {
        let output = output.downcast_ref::<T::Event>().unwrap().clone();
        if let Some(callback) = self.callback.as_ref() {
            callback.invoke(output).await;
        }
    }
}

impl<T: InteractiveComponent + 'static> NodeData<T> {
    pub async fn component<C: Send + Sync + 'static>(&self) -> Option<Handle<C>> {
        let anymap = self.components.read().await;
        anymap.get::<Handle<C>>().cloned()
    }

    pub async fn interactive_component(&self) -> Handle<T> {
        self.component().await.unwrap()
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct NodeData<T>
where
    T: InteractiveComponent,
{
    pub(crate) components: Handle<ThreadsafeAnyMap>,
    #[derivative(Debug = "ignore")]
    callback: Option<Callback<T::Event>>,
    interactive: bool,
    pub(crate) layout: Handle<Layout>,
    pub(crate) style_sheet: Handle<StyleSheet>,
    pub(crate) bounds: Handle<AbsoluteBounds>,
    _phantom: std::marker::PhantomData<T>,
}

#[async_trait]
pub trait NodeDataWindowExt {
    async fn close_requested(&self) -> KludgineResult<CloseResponse>;
}

#[async_trait]
impl<T> NodeDataWindowExt for NodeData<T>
where
    T: Window,
{
    async fn close_requested(&self) -> KludgineResult<CloseResponse> {
        let component = self.interactive_component().await;
        let component = component.read().await;
        component.close_requested().await
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub(crate) component: Handle<Box<dyn AnyNode>>,
}

impl Node {
    pub fn from_components<T: InteractiveComponent + 'static>(
        components: ThreadsafeAnyMap,
        style_sheet: StyleSheet,
        bounds: AbsoluteBounds,
        interactive: bool,
        callback: Option<Callback<T::Event>>,
    ) -> Self {
        let components = Handle::new(components);

        let style_sheet = Handle::new(style_sheet);
        let bounds = Handle::new(bounds);
        Self {
            component: Handle::new(Box::new(NodeData::<T> {
                style_sheet,
                components,
                callback,
                bounds,
                interactive,
                layout: Default::default(),
                _phantom: Default::default(),
            })),
        }
    }
    pub fn new<T: InteractiveComponent + 'static>(
        component: T,
        style_sheet: StyleSheet,
        bounds: AbsoluteBounds,
        interactive: bool,
        callback: Option<Callback<T::Event>>,
    ) -> Self {
        let component = Handle::new(component);
        let mut components = ThreadsafeAnyMap::new();
        components.insert(component);

        Self::from_components::<T>(components, style_sheet, bounds, interactive, callback)
    }

    pub async fn style_sheet(&self) -> StyleSheet {
        let component = self.component.read().await;
        component.style_sheet().await
    }

    pub async fn interactive(&self) -> bool {
        let component = self.component.read().await;
        component.interactive()
    }

    pub async fn bounds(&self) -> AbsoluteBounds {
        let component = self.component.read().await;
        component.bounds().await
    }

    pub async fn set_style_sheet(&self, sheet: StyleSheet) {
        let component = self.component.read().await;
        component.set_style_sheet(sheet).await
    }

    pub async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let component = self.component.read().await;
        component.content_size(context, constraints).await
    }

    pub async fn layout(
        &self,
        context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>> {
        let component = self.component.read().await;
        component.layout(context).await
    }

    /// Called once the Window is opened
    pub async fn initialize(&self, context: &mut SceneContext) -> KludgineResult<()> {
        let component = self.component.read().await;
        component.initialize(context).await
    }

    pub async fn render(&self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        let component = self.component.read().await;
        component.render(context, layout).await
    }

    pub async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        let component = self.component.read().await;
        component.render_background(context, layout).await
    }

    pub async fn update(&self, context: &mut SceneContext) -> KludgineResult<()> {
        let component = self.component.read().await;
        component.update(context).await
    }

    pub async fn hit_test(
        &self,
        context: &mut Context,
        position: Point<f32, Scaled>,
    ) -> KludgineResult<bool> {
        let component = self.component.read().await;
        component.hit_test(context, position).await
    }

    pub async fn mouse_down(
        &self,
        context: &mut Context,
        position: Point<f32, Scaled>,
        button: MouseButton,
    ) -> KludgineResult<EventStatus> {
        let component = self.component.read().await;
        component.mouse_down(context, position, button).await
    }

    pub async fn mouse_wheel(
        &self,
        context: &mut Context,
        delta: MouseScrollDelta,
        touch_phase: TouchPhase,
    ) -> KludgineResult<EventStatus> {
        let component = self.component.read().await;
        component.mouse_wheel(context, delta, touch_phase).await
    }

    pub async fn mouse_drag(
        &self,
        context: &mut Context,
        position: Option<Point<f32, Scaled>>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        let component = self.component.read().await;
        component.mouse_drag(context, position, button).await
    }

    pub async fn mouse_up(
        &self,
        context: &mut Context,
        position: Option<Point<f32, Scaled>>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        let component = self.component.read().await;
        component.mouse_up(context, position, button).await
    }

    pub async fn hovered(&self, context: &mut Context) -> KludgineResult<()> {
        let component = self.component.read().await;
        component.hovered(context).await
    }

    pub async fn unhovered(&self, context: &mut Context) -> KludgineResult<()> {
        let component = self.component.read().await;
        component.unhovered(context).await
    }

    pub async fn callback<Input: Send + Sync + 'static>(&self, message: Input) {
        let component = self.component.clone();
        Runtime::spawn(async move {
            let component = component.read().await;
            component.send_callback(Box::new(message)).await
        })
        .detach();
    }

    pub(crate) async fn set_layout(&self, layout: Layout) {
        let component = self.component.read().await;
        component.set_layout(layout).await
    }

    pub async fn last_layout(&self) -> Layout {
        let component = self.component.read().await;
        component.get_layout().await
    }
}
