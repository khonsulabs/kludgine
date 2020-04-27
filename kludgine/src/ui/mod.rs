use super::{
    math::{Point, Rect, Size},
    scene::SceneTarget,
    style::Style,
    KludgineHandle, KludgineResult,
};
use std::sync::Arc;

pub mod grid;
pub mod label;
pub mod view;
use view::View;
#[derive(Clone)]
pub struct UserInterface {
    handle: KludgineHandle<UserInterfaceData>,
}

impl UserInterface {
    pub fn new(base_style: Style) -> Self {
        Self {
            handle: KludgineHandle::new(UserInterfaceData {
                root: None,
                base_style,
            }),
        }
    }

    pub fn set_root(&self, component: Component) {
        let mut ui = self.handle.write().expect("Error locking UI to write");
        ui.root = Some(component);
    }

    pub fn render(&self, scene: &mut SceneTarget) -> KludgineResult<()> {
        let ui = self.handle.read().expect("Error locking UI to write");
        if let Some(root_component) = &ui.root {
            let root = root_component
                .handle
                .read()
                .expect("Error locking component");
            let mut view = root.controller.view()?;
            view.update_style(scene, &ui.base_style)?;
            view.layout_within(
                scene,
                Rect::sized(
                    Point::new(0.0, 0.0),
                    Size::new(scene.size().width, scene.size().height),
                ),
            )?;
            view.render(scene)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct UserInterfaceData {
    root: Option<Component>,
    base_style: Style,
}

#[derive(Clone, Debug)]
pub struct Component {
    handle: KludgineHandle<ComponentData>,
}
impl Component {
    pub fn new<C: Controller + 'static>(controller: C) -> Component {
        let handle = KludgineHandle::new(ComponentData {
            controller: Arc::new(controller),
            view: None,
        });

        Component { handle }
    }

    pub fn view(&self) -> KludgineResult<Box<dyn View>> {
        let handle = self
            .handle
            .read()
            .expect("Error locking component for read");
        handle.controller.view()
    }
}

#[derive(Debug)]
pub(crate) struct ComponentData {
    controller: Arc<dyn Controller>,
    view: Option<Box<dyn View>>,
}

pub trait Controller: std::fmt::Debug {
    fn view(&self) -> KludgineResult<Box<dyn View>>;
}
