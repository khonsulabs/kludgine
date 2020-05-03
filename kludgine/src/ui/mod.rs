use super::{
    math::{Point, Rect, Size},
    scene::SceneTarget,
    style::Style,
    window::{Event, EventStatus, InputEvent},
    KludgineHandle, KludgineResult,
};
use async_trait::async_trait;
use std::collections::HashSet;
use winit::event::{ElementState, MouseButton};

pub mod grid;
pub mod label;
pub mod view;
use view::View;
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

    pub async fn set_root(&self, component: Component) {
        let mut ui = self.handle.write().await;
        ui.root = Some(component);
    }

    pub async fn render<'a>(&self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        let ui = self.handle.read().await;
        if let Some(root_component) = &ui.root {
            let view_handle = root_component.view().await?;
            let mut view = view_handle.write().await;
            view.update_style(scene, &ui.base_style).await?;
            view.layout_within(
                scene,
                Rect::sized(
                    Point::new(0.0, 0.0),
                    Size::new(scene.size().width, scene.size().height),
                ),
            )
            .await?;
            view.render(scene).await?;
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

#[derive(Debug)]
pub(crate) struct ComponentData {
    controller: Box<dyn Controller>,
    view: Option<KludgineHandle<Box<dyn View>>>,
    hovered_at: Option<Point>,
    last_known_bounds: Rect,
}

impl Component {
    pub fn new<C: Controller + 'static>(controller: C) -> Component {
        let handle = KludgineHandle::new(ComponentData {
            controller: Box::new(controller),
            view: None,
            hovered_at: None,
            last_known_bounds: Rect::default(),
        });

        Component { handle }
    }

    async fn view(&self) -> KludgineResult<KludgineHandle<Box<dyn View>>> {
        let mut handle = self.handle.write().await;
        let view = match handle.view.as_ref() {
            Some(view) => view.clone(),
            None => {
                let view = handle.controller.view().await?;
                handle.view = Some(view.clone());
                view
            }
        };

        Ok(view)
    }

    pub async fn mouse_moved(
        &self,
        window_position: Point,
    ) -> KludgineResult<ComponentEventStatus> {
        let handled = {
            // Talk to the controller before updating the view
            let mut component = self.handle.write().await;
            let mut handled = ComponentEventStatus::ignored();
            if component.hovered_at.is_none() {
                handled.update_with(component.controller.mouse_entered(self).await?);
            }
            let status = component
                .controller
                .mouse_moved(self, window_position)
                .await?;
            if status.rebuild_view {
                component.view = None;
            }
            status
        };

        // Update the view's hover information
        let view_handle = self.view().await?;
        let mut view = view_handle.write().await;
        if view.bounds().contains(window_position) {
            view.hovered_at(window_position).await?
        } else if view.base_view().mouse_status.is_some() {
            view.unhovered().await?
        }
        Ok(handled)
    }

    pub async fn mouse_exited(&self) -> KludgineResult<ComponentEventStatus> {
        let handled = {
            // Talk to the controller before updating the view
            let mut component = self.handle.write().await;
            let status = component.controller.mouse_exited(self).await?;
            if status.rebuild_view {
                component.view = None;
            }
            status
        };

        // Update the view's hover information
        let view_handle = self.view().await?;
        let mut view = view_handle.write().await;
        if view.base_view().mouse_status.is_some() {
            view.unhovered().await?
        }
        Ok(handled)
    }

    pub async fn mouse_button_down(
        &self,
        button: MouseButton,
        window_position: Point,
    ) -> KludgineResult<ComponentEventStatus> {
        let handled = {
            // Talk to the controller before updating the view
            let mut component = self.handle.write().await;
            let status = component
                .controller
                .mouse_button_down(self, button, window_position)
                .await?;
            if status.rebuild_view {
                component.view = None;
            }
            status
        };

        let view_handle = self.view().await?;
        let mut view = view_handle.write().await;
        if view.bounds().contains(window_position) {
            view.activated_at(window_position).await?
        } else if view.base_view().mouse_status.is_some() {
            view.deactivated().await?
        }
        Ok(handled)
    }

    pub async fn mouse_button_up(
        &self,
        button: MouseButton,
        window_position: Point,
    ) -> KludgineResult<ComponentEventStatus> {
        let handled = {
            // Talk to the controller before updating the view
            let mut component = self.handle.write().await;
            let status = component
                .controller
                .mouse_button_up(self, button, window_position)
                .await?;
            if status.rebuild_view {
                component.view = None;
            }
            status
        };

        let view_handle = self.view().await?;
        let mut view = view_handle.write().await;
        if view.base_view().mouse_status.is_some() {
            view.deactivated().await?
        }
        Ok(handled)
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
    async fn view(&self) -> KludgineResult<KludgineHandle<Box<dyn View>>>;
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
    // TODO add button tracking to provide click events
    // async fn clicked_at(
    //     &mut self,
    //     _button: MouseButton,
    //     _window_position: Point,
    // ) -> KludgineResult<ComponentEventStatus> {
    //     Ok(ComponentEventStatus::ignored())
    // }
}

#[derive(Debug)]
pub struct ViewController {
    view: KludgineHandle<Box<dyn View>>,
}

impl ViewController {
    pub fn new(view: KludgineHandle<Box<dyn View>>) -> Self {
        Self { view }
    }
}

#[async_trait]
impl Controller for ViewController {
    async fn view(&self) -> KludgineResult<KludgineHandle<Box<dyn View>>> {
        Ok(self.view.clone())
    }
}
