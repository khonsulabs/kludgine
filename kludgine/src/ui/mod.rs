use super::{
    math::{Point, Rect, Size},
    scene::SceneTarget,
    style::Style,
    KludgineHandle, KludgineResult,
};
use async_std::sync::RwLock;
use async_trait::async_trait;
use std::sync::Arc;

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
            handle: Arc::new(RwLock::new(UserInterfaceData {
                root: None,
                base_style,
                hover: None,
            })),
        }
    }

    pub async fn set_root(&self, component: Component) {
        let mut ui = self.handle.write().await;
        ui.root = Some(component);
    }

    pub async fn render<'a>(&self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        let ui = self.handle.read().await;
        if let Some(root_component) = &ui.root {
            let root = root_component.handle.read().await;
            let mut view = root.controller.view().await?;
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
}

#[derive(Clone, Debug)]
pub struct Component {
    handle: KludgineHandle<ComponentData>,
}

#[derive(Debug)]
pub(crate) struct ComponentData {
    controller: Box<dyn Controller>,
    view: Option<Box<dyn View>>,
}

impl Component {
    pub fn new<C: Controller + 'static>(controller: C) -> Component {
        let handle = Arc::new(RwLock::new(ComponentData {
            controller: Box::new(controller),
            view: None,
        }));

        Component { handle }
    }

    async fn view(&self) -> KludgineResult<Box<dyn View>> {
        let handle = self.handle.read().await;
        handle.controller.view().await
    }
}

#[async_trait]
pub trait Controller: std::fmt::Debug + Sync + Send + 'static {
    async fn view(&self) -> KludgineResult<Box<dyn View>>;
}
