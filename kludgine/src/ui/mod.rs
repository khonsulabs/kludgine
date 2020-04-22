use super::{scene::Scene, KludgineHandle, KludgineResult};
use async_trait::async_trait;
use crossbeam::sync::ShardedLock;
use generational_arena::{Arena, Index};
use kludgine_macros::View;
use std::collections::HashMap;
use std::sync::Weak;
use stretch::style::Style;

#[derive(Clone)]
pub struct UserInterface {
    handle: KludgineHandle<UserInterfaceData>,
}

impl UserInterface {
    pub fn new() -> Self {
        Self {
            handle: KludgineHandle::new(UserInterfaceData {
                arena: Arena::new(),
                hierarchy: HashMap::new(),
                root: None,
            }),
        }
    }

    pub fn create_component<C: Controller + 'static>(&self, controller: C) -> Component {
        let handle = KludgineHandle::new(ComponentData {
            ui: self.handle.downgrade(),
            controller: Box::new(controller),
        });

        let mut ui = self.handle.write().expect("Error locking UI to write");
        let id = ui.arena.insert(handle.clone());

        Component { id, handle }
    }

    pub fn set_root(&self, component: &Component) {
        let mut ui = self.handle.write().expect("Error locking UI to write");
        ui.root = Some(component.id);
    }

    pub async fn render(&self, scene: &mut Scene) -> KludgineResult<()> {
        let ui = self.handle.read().expect("Error locking UI to write");
        if let Some(id) = ui.root {
            let root = ui
                .arena
                .get(id)
                .unwrap()
                .read()
                .expect("Error locking component");
            let view = root.controller.view().await?;
            self.update_layout();
            todo!()
        }
        Ok(())
    }
}

pub(crate) struct UserInterfaceData {
    arena: Arena<KludgineHandle<ComponentData>>,
    root: Option<Index>,
    hierarchy: HashMap<Index, Vec<Index>>,
}

pub struct Component {
    id: Index,
    handle: KludgineHandle<ComponentData>,
}

pub(crate) struct ComponentData {
    ui: Weak<ShardedLock<UserInterfaceData>>,
    controller: Box<dyn Controller>,
}

#[async_trait]
pub trait Controller {
    async fn view(&self) -> KludgineResult<Box<dyn View>>;
}

#[async_trait]
pub trait View {
    async fn render(&self, &mut scene: Scene) -> KludgineResult<()>;
}

pub struct BaseView {
    style: Style,
}

#[async_trait]
pub trait ViewCore {
    async fn style(&self) -> Style;
}

#[derive(View)]
pub struct Label {
    view: BaseView,
    value: String,
}

// Component -> Controller
//   Controller -> View
// Component render view
