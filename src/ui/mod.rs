use super::{
    math::{Point, Rect, Size},
    scene::SceneTarget,
    style::{EffectiveStyle, Layout, Style},
    window::{Event, EventStatus, InputEvent},
    KludgineHandle, KludgineResult,
};
use async_trait::async_trait;
use std::collections::HashSet;
use winit::event::{ElementState, MouseButton};

pub mod grid;
pub mod label;
pub mod view;
use view::{BaseView, MouseStatus};
#[derive(Clone)]
pub struct UserInterface {
    handle: KludgineHandle<UserInterfaceData>,
}

#[derive(Debug)]
pub(crate) struct UserInterfaceData {
    root: Option<Component>,
    base_style: Style,
    hover: Option<Component>,
    last_mouse_position: Option<Point>,
    down_mouse_buttons: HashSet<MouseButton>,
}

impl UserInterface {
    pub fn new(base_style: Style) -> Self {
        Self {
            handle: KludgineHandle::new(UserInterfaceData {
                root: None,
                base_style,
                hover: None,
                last_mouse_position: None,
                down_mouse_buttons: HashSet::new(),
            }),
        }
    }
    /// MOVE THE BASE VIEW INTO COMPONENT AND MAKE THE COMPONENT MANAGE LAYOUT
    /// Make a render function that takes the scene and bounds

    pub async fn set_root(&self, component: Component) {
        let mut ui = self.handle.write().await;
        ui.root = Some(component);
    }

    pub async fn render(&self, scene: &mut SceneTarget<'_>) -> KludgineResult<()> {
        let ui = self.handle.read().await;
        if let Some(root_component) = &ui.root {
            root_component.update_style(scene, &ui.base_style).await?;
            root_component
                .layout_within(
                    scene,
                    Rect::sized(
                        Point::new(0.0, 0.0),
                        Size::new(scene.size().width, scene.size().height),
                    ),
                )
                .await?;
            root_component.render(scene).await?;
        }
        Ok(())
    }

    pub async fn process_input(
        &self,
        input_event: InputEvent,
    ) -> KludgineResult<ComponentEventStatus> {
        match input_event.event {
            Event::MouseMoved { position } => self.update_mouse_position(position).await,
            Event::MouseButton { button, state } => match state {
                ElementState::Pressed => self.mouse_down(button).await,
                ElementState::Released => self.mouse_up(button).await,
            },
            _ => Ok(ComponentEventStatus::ignored()),
        }
    }

    async fn update_mouse_position(
        &self,
        position: Option<Point>,
    ) -> KludgineResult<ComponentEventStatus> {
        {
            let mut ui = self.handle.write().await;
            ui.last_mouse_position = position;
        }
        let ui = self.handle.write().await;
        let root = ui.root.as_ref().unwrap();
        match position {
            Some(position) => root.mouse_moved(position).await,
            None => root.mouse_exited().await,
        }
    }

    async fn mouse_down(&self, button: MouseButton) -> KludgineResult<ComponentEventStatus> {
        let mut ui = self.handle.write().await;
        ui.down_mouse_buttons.insert(button);

        let root = ui.root.as_ref().unwrap();
        let mut handled = ComponentEventStatus::ignored();
        if let Some(window_position) = ui.last_mouse_position {
            handled.update_with(root.mouse_button_down(button, window_position).await?);
        }
        Ok(handled)
    }

    async fn mouse_up(&self, button: MouseButton) -> KludgineResult<ComponentEventStatus> {
        let mut ui = self.handle.write().await;
        ui.down_mouse_buttons.insert(button);

        let root = ui.root.as_ref().unwrap();
        let mut handled = ComponentEventStatus::ignored();
        if let Some(window_position) = ui.last_mouse_position {
            handled.update_with(root.mouse_button_up(button, window_position).await?);
        }
        Ok(handled)
    }
}

#[derive(Clone, Debug)]
pub struct Component {
    handle: KludgineHandle<ComponentData>,
}

#[derive(Debug)]
pub(crate) struct ComponentData {
    controller: Box<dyn Controller>,
    base_view: BaseView,
    hovered_at: Option<Point>,
    last_known_bounds: Rect,
}

#[derive(Default, Debug)]
pub struct ComponentEventStatus {
    handled: EventStatus,
    rebuild_view: bool,
}

impl From<EventStatus> for ComponentEventStatus {
    fn from(handled: EventStatus) -> Self {
        Self {
            handled,
            rebuild_view: false,
        }
    }
}

impl ComponentEventStatus {
    pub fn update_with<S: Into<ComponentEventStatus>>(&mut self, other: S) {
        let other = other.into();
        self.handled.update_with(other.handled);
        self.rebuild_view = self.rebuild_view || other.rebuild_view;
    }
}

impl ComponentEventStatus {
    pub fn ignored() -> Self {
        Self {
            handled: EventStatus::Ignored,
            rebuild_view: false,
        }
    }

    pub fn processed() -> Self {
        Self {
            handled: EventStatus::Ignored,
            rebuild_view: false,
        }
    }

    pub fn rebuild_view_ignored() -> Self {
        Self {
            handled: EventStatus::Ignored,
            rebuild_view: true,
        }
    }

    pub fn rebuild_view_processed() -> Self {
        Self {
            handled: EventStatus::Processed,
            rebuild_view: true,
        }
    }
}

impl Component {
    pub fn new<C: Controller + 'static>(controller: C) -> Component {
        let handle = KludgineHandle::new(ComponentData {
            controller: Box::new(controller),
            hovered_at: None,
            last_known_bounds: Rect::default(),
            base_view: BaseView::default(),
        });

        Component { handle }
    }

    pub async fn mouse_moved(
        &self,
        window_position: Point,
    ) -> KludgineResult<ComponentEventStatus> {
        // Talk to the controller before updating the view
        let mut component = self.handle.write().await;
        let mut handled = ComponentEventStatus::ignored();
        if component.hovered_at.is_none() {
            handled.update_with(component.controller.mouse_entered(self).await?);
        }
        handled.update_with(
            component
                .controller
                .mouse_moved(self, window_position)
                .await?,
        );

        // Update the view's hover information
        if component.base_view.bounds.contains(window_position) {
            component.base_view.hovered_at(window_position)?;
        } else if component.base_view.mouse_status.is_some() {
            component.base_view.unhovered()?;
        }
        Ok(handled)
    }

    pub async fn mouse_exited(&self) -> KludgineResult<ComponentEventStatus> {
        // Talk to the controller before updating the view
        let mut component = self.handle.write().await;
        let status = component.controller.mouse_exited(self).await?;

        if component.base_view.mouse_status.is_some() {
            component.base_view.unhovered()?;
        }
        Ok(status)
    }

    pub async fn mouse_button_down(
        &self,
        button: MouseButton,
        window_position: Point,
    ) -> KludgineResult<ComponentEventStatus> {
        // Talk to the controller before updating the view
        let mut component = self.handle.write().await;
        let status = component
            .controller
            .mouse_button_down(self, button, window_position)
            .await?;

        if component.base_view.bounds.contains(window_position) {
            component.base_view.activated_at(window_position)?;
        } else if component.base_view.mouse_status.is_some() {
            component.base_view.deactivated()?;
        }
        Ok(status)
    }

    pub async fn mouse_button_up(
        &self,
        button: MouseButton,
        window_position: Point,
    ) -> KludgineResult<ComponentEventStatus> {
        // Talk to the controller before updating the view
        let mut component = self.handle.write().await;
        let status = component
            .controller
            .mouse_button_up(self, button, window_position)
            .await?;

        if component.base_view.mouse_status.is_some() {
            component.base_view.deactivated()?;
        }
        Ok(status)
    }

    async fn layout_within(&self, scene: &mut SceneTarget<'_>, bounds: Rect) -> KludgineResult<()> {
        let mut component = self.handle.write().await;
        let size = self
            .content_size(&component.base_view.bounds.size, scene)
            .await?;
        component.base_view.layout_within(&size, bounds)?;
        component
            .controller
            .layout_within(self, scene, bounds)
            .await
    }

    async fn content_size(
        &self,
        maximum_size: &Size,
        scene: &mut SceneTarget<'_>,
    ) -> KludgineResult<Size> {
        let component = self.handle.read().await;
        component
            .controller
            .content_size(self, maximum_size, scene)
            .await
    }

    async fn update_style(
        &self,
        scene: &mut SceneTarget<'_>,
        inherited_style: &Style,
    ) -> KludgineResult<()> {
        let mut component = self.handle.write().await;
        if component
            .controller
            .update_style(self, scene, inherited_style)
            .await?
            == EventStatus::Ignored
        {
            self.compute_effective_style(inherited_style, scene).await;
        }
        Ok(())
    }

    async fn render(&self, scene: &mut SceneTarget<'_>) -> KludgineResult<()> {
        let component = self.handle.read().await;
        component.controller.render(self, scene).await
    }

    pub async fn bounds(&self) -> Rect {
        let component = self.handle.read().await;
        component.base_view.bounds
    }

    pub async fn layout(&self) -> Layout {
        let component = self.handle.read().await;
        component.base_view.layout.clone()
    }

    pub async fn effective_style(&self) -> EffectiveStyle {
        let component = self.handle.read().await;
        component.base_view.effective_style.clone()
    }

    pub async fn set_style(&mut self, style: Style) {
        let mut component = self.handle.write().await;
        component.base_view.style = style;
    }
    pub async fn set_hover_style(&mut self, style: Style) {
        let mut component = self.handle.write().await;
        component.base_view.hover_style = style;
    }

    pub async fn set_layout(&mut self, layout: Layout) {
        let mut component = self.handle.write().await;
        component.base_view.layout = layout;
    }

    pub async fn compute_effective_style(
        &self,
        inherited_style: &Style,
        scene: &mut SceneTarget<'_>,
    ) -> Style {
        let current_style = self.current_style().await.inherit_from(inherited_style);
        let mut component = self.handle.write().await;
        component.base_view.effective_style = current_style.effective_style(scene);
        current_style
    }

    pub async fn current_style(&self) -> Style {
        let component = self.handle.read().await;
        match &component.base_view.mouse_status {
            Some(mouse_status) => match mouse_status {
                MouseStatus::Hovered(_) => component
                    .base_view
                    .hover_style
                    .inherit_from(&component.base_view.style),
                MouseStatus::Activated(_) => component.base_view.activated_style.inherit_from(
                    &component
                        .base_view
                        .hover_style
                        .inherit_from(&component.base_view.style),
                ),
            },
            None => component.base_view.style.clone(),
        }
    }
    // pub async fn clicked(
    //     &self,
    //     button: MouseButton,
    //     window_position: Point,
    // ) -> KludgineResult<EventStatus> {
    //     let view_handle = self.view().await?;
    //     let mut view = view_handle.write().await;
    //     let mut component = self.handle.write().await;
    //     let mut handled = EventStatus::Ignored;
    //     if view.bounds().contains(window_position) {
    //         handled.update_with(
    //             component
    //                 .controller
    //                 .clicked_at(button, window_position)
    //                 .await?,
    //         );
    //         view.activated_at(window_position).await?
    //     } else if view.base_view().mouse_status.is_some() {
    //         handled.update_with(component.controller.mouse_exited().await?);
    //         view.deactivated().await?
    //     }
    //     Ok(handled)
    // }
}

#[async_trait]
pub trait Controller: std::fmt::Debug + Sync + Send + 'static {
    async fn render(
        &self,
        _component: &Component,
        _scene: &mut SceneTarget<'_>,
    ) -> KludgineResult<()>;

    async fn update_style(
        &mut self,
        _component: &Component,
        _scene: &mut SceneTarget<'_>,
        _inherited_style: &Style,
    ) -> KludgineResult<EventStatus> {
        Ok(EventStatus::Ignored)
    }
    async fn mouse_exited(
        &mut self,
        _component: &Component,
    ) -> KludgineResult<ComponentEventStatus> {
        Ok(ComponentEventStatus::ignored())
    }
    async fn mouse_entered(
        &mut self,
        _component: &Component,
    ) -> KludgineResult<ComponentEventStatus> {
        Ok(ComponentEventStatus::ignored())
    }
    async fn mouse_moved(
        &mut self,
        _component: &Component,
        _window_location: Point,
    ) -> KludgineResult<ComponentEventStatus> {
        Ok(ComponentEventStatus::ignored())
    }
    async fn mouse_button_down(
        &mut self,
        _component: &Component,
        _button: MouseButton,
        __window_position: Point,
    ) -> KludgineResult<ComponentEventStatus> {
        Ok(ComponentEventStatus::ignored())
    }
    async fn mouse_button_up(
        &mut self,
        _component: &Component,
        _button: MouseButton,
        _window_position: Point,
    ) -> KludgineResult<ComponentEventStatus> {
        Ok(ComponentEventStatus::ignored())
    }

    async fn layout_within(
        &mut self,
        _component: &Component,
        _scene: &mut SceneTarget<'_>,
        _bounds: Rect,
    ) -> KludgineResult<()> {
        Ok(())
    }

    async fn content_size(
        &self,
        _component: &Component,
        _maximum_size: &Size,
        _scene: &mut SceneTarget<'_>,
    ) -> KludgineResult<Size>;
    // TODO add button tracking to provide click events
    // async fn clicked_at(
    //     &mut self,
    //     _button: MouseButton,
    //     _window_position: Point,
    // ) -> KludgineResult<ComponentEventStatus> {
    //     Ok(ComponentEventStatus::ignored())
    // }
}
