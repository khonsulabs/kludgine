use super::{
    math::{Dimension, Point, Rect, Size, Surround},
    scene::{Scene, SceneTarget},
    style::{Color, EffectiveStyle, Layout, Style, Weight},
    text::{Text, TextWrap},
    KludgineError, KludgineHandle, KludgineResult,
};
use crossbeam::sync::ShardedLock;
use generational_arena::{Arena, Index};
use kludgine_macros::ViewCore;
use std::collections::HashMap;
use std::sync::{Arc, Weak};

#[derive(Clone)]
pub struct UserInterface {
    handle: KludgineHandle<UserInterfaceData>,
}

impl UserInterface {
    pub fn new(base_style: Style) -> Self {
        Self {
            handle: KludgineHandle::new(UserInterfaceData {
                arena: Arena::new(),
                hierarchy: HashMap::new(),
                root: None,
                base_style,
            }),
        }
    }

    pub fn create_component<C: Controller + 'static>(&self, controller: C) -> Component {
        let handle = KludgineHandle::new(ComponentData {
            controller: Arc::new(controller),
            ui: self.handle.downgrade(),
            view: None,
        });

        let mut ui = self.handle.write().expect("Error locking UI to write");
        let id = ui.arena.insert(handle.clone());

        Component { id, handle }
    }

    pub fn set_root(&self, component: &Component) {
        let mut ui = self.handle.write().expect("Error locking UI to write");
        ui.root = Some(component.id);
    }

    pub fn render(&self, scene: &mut SceneTarget) -> KludgineResult<()> {
        let ui = self.handle.read().expect("Error locking UI to write");
        if let Some(id) = ui.root {
            let root = ui
                .arena
                .get(id)
                .unwrap()
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

pub(crate) struct UserInterfaceData {
    arena: Arena<KludgineHandle<ComponentData>>,
    root: Option<Index>,
    hierarchy: HashMap<Index, Vec<Index>>,
    base_style: Style,
}

pub struct Component {
    id: Index,
    handle: KludgineHandle<ComponentData>,
}

pub(crate) struct ComponentData {
    ui: Weak<ShardedLock<UserInterfaceData>>,
    controller: Arc<dyn Controller>,
    view: Option<Box<dyn View>>,
}

pub trait Controller {
    fn view(&self) -> KludgineResult<Box<dyn View>>;
}

pub trait View {
    fn render(&self, scene: &mut SceneTarget) -> KludgineResult<()>;
    fn layout_within(&mut self, scene: &mut SceneTarget, bounds: Rect) -> KludgineResult<()>;
    fn update_style(
        &mut self,
        scene: &mut SceneTarget,
        inherited_style: &Style,
    ) -> KludgineResult<()>;
    fn content_size(&self, bounds: &Rect, scene: &mut SceneTarget) -> KludgineResult<Size>;
}

#[derive(Default, Clone)]
pub struct BaseView {
    style: Style,
    effective_style: EffectiveStyle,
    layout: Layout,
    bounds: Rect,
}

impl BaseView {
    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    pub fn layout_within(&mut self, content_size: &Size, bounds: Rect) -> KludgineResult<()> {
        self.bounds = self.layout.compute_bounds(content_size, bounds);
        Ok(())
    }
}

pub trait ViewCore: View {
    fn base_view(&self) -> &BaseView;
    fn base_view_mut(&mut self) -> &mut BaseView;
}

pub trait ViewBuilder {
    fn build(&self) -> KludgineResult<Box<dyn View>>;
}

impl<T> ViewBuilder for T
where
    T: View + Clone + Sized + 'static,
{
    fn build(&self) -> KludgineResult<Box<dyn View>> {
        Ok(Box::new(self.clone()))
    }
}

pub trait ViewCoreBuilder {
    fn with_style(&mut self, style: Style) -> &mut Self;
    fn with_font_family(&mut self, font_family: String) -> &mut Self;
    fn with_font_size(&mut self, font_size: f32) -> &mut Self;
    fn with_font_weight(&mut self, font_weight: Weight) -> &mut Self;
    fn with_color(&mut self, color: Color) -> &mut Self;
    fn with_layout(&mut self, layout: Layout) -> &mut Self;
    fn with_location(&mut self, location: Point) -> &mut Self;
    fn with_margin<D: Into<Dimension>>(&mut self, margin: Surround<D>) -> &mut Self;
    fn with_padding<D: Into<Dimension>>(&mut self, padding: Surround<D>) -> &mut Self;
    fn with_border<D: Into<Dimension>>(&mut self, border: Surround<D>) -> &mut Self;
}

impl<T> ViewCoreBuilder for T
where
    T: ViewCore,
{
    fn with_style(&mut self, style: Style) -> &mut Self {
        self.base_view_mut().style = style;
        self
    }

    fn with_font_family(&mut self, font_family: String) -> &mut Self {
        self.base_view_mut().style.font_family = Some(font_family.clone());
        self
    }

    fn with_font_size(&mut self, font_size: f32) -> &mut Self {
        self.base_view_mut().style.font_size = Some(font_size);
        self
    }

    fn with_font_weight(&mut self, font_weight: Weight) -> &mut Self {
        self.base_view_mut().style.font_weight = Some(font_weight);
        self
    }

    fn with_color(&mut self, color: Color) -> &mut Self {
        self.base_view_mut().style.color = Some(color);
        self
    }

    fn with_layout(&mut self, layout: Layout) -> &mut Self {
        self.base_view_mut().layout = layout;
        self
    }

    fn with_location(&mut self, location: Point) -> &mut Self {
        self.base_view_mut().layout.location = location;
        self
    }

    fn with_margin<D: Into<Dimension>>(&mut self, margin: Surround<D>) -> &mut Self {
        self.base_view_mut().layout.margin = margin.into_dimensions();
        self
    }

    fn with_padding<D: Into<Dimension>>(&mut self, padding: Surround<D>) -> &mut Self {
        self.base_view_mut().layout.padding = padding.into_dimensions();
        self
    }

    fn with_border<D: Into<Dimension>>(&mut self, border: Surround<D>) -> &mut Self {
        self.base_view_mut().layout.border = border.into_dimensions();
        self
    }
}

#[derive(ViewCore, Default, Clone)]
pub struct Label {
    view: BaseView,
    value: Option<String>,
}

impl View for Label {
    fn render(&self, scene: &mut SceneTarget) -> KludgineResult<()> {
        let font = scene.lookup_font(
            &self.view.effective_style.font_family,
            self.view.effective_style.font_weight,
        )?;
        let metrics = font.metrics(self.view.effective_style.font_size);
        match self.create_text()? {
            Some(text) => text.render_at(
                scene,
                Point::new(
                    self.view.bounds.origin.x,
                    self.view.bounds.origin.y + metrics.ascent / scene.effective_scale_factor(),
                ),
                self.wrapping(&self.view.bounds.size),
            ),
            None => Ok(()),
        }
    }

    fn update_style(
        &mut self,
        scene: &mut SceneTarget,
        inherited_style: &Style,
    ) -> KludgineResult<()> {
        let inherited_style = self.view.style.inherit_from(&inherited_style);
        self.view.effective_style = inherited_style.effective_style(scene);
        Ok(())
    }

    fn layout_within(&mut self, scene: &mut SceneTarget, bounds: Rect) -> KludgineResult<()> {
        self.view
            .layout_within(&self.content_size(&bounds, scene)?, bounds)
    }

    fn content_size(&self, bounds: &Rect, scene: &mut SceneTarget) -> KludgineResult<Size> {
        let size = match self.create_text()? {
            Some(text) => {
                text.wrap(
                    scene,
                    self.wrapping(&self.view.layout.size_with_minimal_padding(&bounds.size)),
                )?
                .size()
                    / scene.effective_scale_factor()
            }
            None => Size::default(),
        };
        Ok(size)
    }
}

impl Label {
    pub fn with_value<S: Into<String>>(&mut self, value: S) -> &mut Self {
        self.value = Some(value.into());
        self
    }

    fn create_text(&self) -> KludgineResult<Option<Text>> {
        if let Some(value) = &self.value {
            Ok(Some(Text::span(value, &self.view.effective_style)))
        } else {
            Ok(None)
        }
    }

    fn wrapping(&self, size: &Size) -> TextWrap {
        TextWrap::SingleLine {
            max_width: size.width,
            truncate: true,
        }
    }
}
// Component -> Controller
//   Controller -> View
// Component render view
