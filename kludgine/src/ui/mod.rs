use super::{
    math::{Point, Rect, Size},
    scene::SceneTarget,
    style::Style,
    window::{Event, EventStatus, InputEvent},
    KludgineHandle, KludgineResult,
};
use async_trait::async_trait;

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
}

impl UserInterface {
    pub fn new(base_style: Style) -> Self {
        Self {
            handle: KludgineHandle::new(UserInterfaceData {
                root: None,
                base_style,
                hover: None,
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

    pub async fn process_input(&self, input_event: InputEvent) -> KludgineResult<EventStatus> {
        match input_event.event {
            Event::MouseMoved { position } => self.update_mouse_position(position).await,
            _ => Ok(EventStatus::Ignored),
        }
    }

    async fn update_mouse_position(&self, position: Option<Point>) -> KludgineResult<EventStatus> {
        match position {
            Some(position) => {
                let ui = self.handle.read().await;
                let root = ui.root.as_ref().unwrap();
                root.mouse_moved(position).await
            }
            None => {
                self.mouse_exited().await?;
                Ok(EventStatus::Ignored)
            }
        }
    }

    async fn mouse_exited(&self) -> KludgineResult<EventStatus> {
        let ui = self.handle.write().await;
        let root = ui.root.as_ref().unwrap().handle.write().await;
        root.controller.mouse_exited().await
    }
}

#[derive(Clone, Debug)]
pub struct Component {
    handle: KludgineHandle<ComponentData>,
}

#[derive(Debug)]
pub(crate) struct ComponentData {
    controller: Box<dyn Controller>,
    view: Option<KludgineHandle<Box<dyn View>>>,
    hovered_at: Option<Point>,
}

impl Component {
    pub fn new<C: Controller + 'static>(controller: C) -> Component {
        let handle = KludgineHandle::new(ComponentData {
            controller: Box::new(controller),
            view: None,
            hovered_at: None,
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

    pub async fn mouse_moved(&self, window_position: Point) -> KludgineResult<EventStatus> {
        let view_handle = self.view().await?;
        let mut view = view_handle.write().await;
        if view.bounds().contains(window_position) {
            view.hovered_at(window_position).await?
        } else if view.base_view().mouse_status.is_some() {
            view.unhovered().await?
        }
        Ok(EventStatus::Ignored)
    }
}

#[async_trait]
pub trait Controller: std::fmt::Debug + Sync + Send + 'static {
    async fn view(&self) -> KludgineResult<KludgineHandle<Box<dyn View>>>;
    async fn mouse_exited(&self) -> KludgineResult<EventStatus> {
        Ok(EventStatus::Ignored)
    }
}
