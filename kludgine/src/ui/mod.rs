use super::{
    math::{Point, Rect, Size},
    scene::Scene,
    text::Font,
    KludgineError, KludgineHandle, KludgineResult,
};
use crossbeam::sync::ShardedLock;
use generational_arena::{Arena, Index};
use kludgine_macros::ViewCore;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use ttf_parser::Weight;

pub mod style;
use style::{Layout, Style};
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
                fonts: HashMap::new(),
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

    pub fn render(&self, scene: &mut Scene) -> KludgineResult<()> {
        let ui = self.handle.read().expect("Error locking UI to write");
        if let Some(id) = ui.root {
            let root = ui
                .arena
                .get(id)
                .unwrap()
                .read()
                .expect("Error locking component");
            let mut view = root.controller.view()?;
            view.layout_within(Rect::sized(
                0.0,
                0.0,
                scene.size().width,
                scene.size().height,
            ))?;
            view.render(scene, self, &ui.base_style)?;
        }
        Ok(())
    }

    pub fn register_font(&self, font: &Font) {
        let family = font.family().expect("Unable to register VecFonts");
        let mut ui = self.handle.write().expect("Error locking UI to read");
        ui.fonts
            .entry(family)
            .and_modify(|fonts| fonts.push(font.clone()))
            .or_insert_with(|| vec![font.clone()]);
    }

    #[cfg(feature = "bundled-fonts-enabled")]
    pub fn register_bundled_fonts(&self) {
        #[cfg(feature = "bundled-fonts-roboto")]
        {
            self.register_font(&crate::text::bundled_fonts::ROBOTO);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_ITALIC);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_BLACK);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_BLACK_ITALIC);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_BOLD);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_BOLD_ITALIC);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_LIGHT);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_LIGHT_ITALIC);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_MEDIUM);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_MEDIUM_ITALIC);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_THIN);
            self.register_font(&crate::text::bundled_fonts::ROBOTO_THIN_ITALIC);
        }
    }

    pub fn lookup_font(&self, family: &str, weight: Weight) -> KludgineResult<Font> {
        let ui = self.handle.read().expect("Error locking UI to read");
        let family = if family.eq_ignore_ascii_case("sans-serif") {
            "Roboto"
        } else {
            family
        };
        match ui.fonts.get(family) {
            Some(fonts) => {
                let mut closest_font = None;
                let mut closest_weight = None;

                for font in fonts.iter() {
                    if font.weight() == weight {
                        return Ok(font.clone());
                    } else {
                        let delta =
                            (font.weight().to_number() as i32 - weight.to_number() as i32).abs();
                        if closest_weight.is_none() || closest_weight.unwrap() > delta {
                            closest_weight = Some(delta);
                            closest_font = Some(font.clone());
                        }
                    }
                }

                closest_font.ok_or_else(|| KludgineError::FontFamilyNotFound(family.to_owned()))
            }
            None => Err(KludgineError::FontFamilyNotFound(family.to_owned())),
        }
    }
}

pub(crate) struct UserInterfaceData {
    arena: Arena<KludgineHandle<ComponentData>>,
    root: Option<Index>,
    hierarchy: HashMap<Index, Vec<Index>>,
    fonts: HashMap<String, Vec<Font>>,
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
    fn render(
        &self,
        scene: &mut Scene,
        ui: &UserInterface,
        inherited_style: &Style,
    ) -> KludgineResult<()>;
    fn layout_within(&mut self, bounds: Rect) -> KludgineResult<()>;
    fn content_size(&self) -> KludgineResult<Size>;
}

#[derive(Default, Clone)]
pub struct BaseView {
    style: Style,
    layout: Layout,
    bounds: Rect,
}

impl BaseView {
    pub fn set_style(&mut self, style: Style) {
        self.style = style;
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
    fn with_font(&mut self, font: String) -> &mut Self;
}

impl<T> ViewCoreBuilder for T
where
    T: ViewCore,
{
    fn with_style(&mut self, style: Style) -> &mut Self {
        self.base_view_mut().style = style;
        self
    }
    fn with_font(&mut self, font: String) -> &mut Self {
        self.base_view_mut().style.font = Some(font.clone());
        self
    }
}

#[derive(ViewCore, Default, Clone)]
pub struct Label {
    view: BaseView,
    value: Option<String>,
}

impl View for Label {
    fn render(
        &self,
        scene: &mut Scene,
        ui: &UserInterface,
        inherited_style: &Style,
    ) -> KludgineResult<()> {
        if let Some(value) = &self.value {
            let inherited_style = self.view.style.inherit_from(&inherited_style);
            let effective_style = inherited_style.effective_style();
            let font = ui.lookup_font(&effective_style.font_family, effective_style.font_weight)?;
            let size = self.view.style.font_size.unwrap_or(12.0);
            let metrics = font.metrics(size);
            scene.render_text_at(
                value,
                &font, // TODO Font fallback
                self.view.style.font_size.unwrap_or(size),
                self.view.style.color.expect("no color"),
                Point::new(self.view.bounds.x1, self.view.bounds.y1 + metrics.ascent),
                Some(self.view.bounds.width()),
            );
        }
        Ok(())
    }

    fn layout_within(&mut self, bounds: Rect) -> KludgineResult<()> {
        self.view.bounds = bounds;
        Ok(())
    }

    fn content_size(&self) -> KludgineResult<Size> {
        todo!()
    }
}

impl Label {
    pub fn with_value<S: Into<String>>(&mut self, value: S) -> &mut Self {
        self.value = Some(value.into());
        self
    }
}
// Component -> Controller
//   Controller -> View
// Component render view
