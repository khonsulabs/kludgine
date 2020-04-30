use crate::{
    math::{Dimension, Point, Rect, Size, Surround},
    scene::SceneTarget,
    style::{Color, EffectiveStyle, Layout, Style, Weight},
    KludgineHandle, KludgineResult,
};
use async_std::sync::RwLock;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum MouseStatus {
    Hovered(Point),
}

#[async_trait]
pub trait View: ViewCore + Send {
    async fn render<'a>(&self, scene: &mut SceneTarget<'a>) -> KludgineResult<()>;

    async fn layout_within<'a>(
        &mut self,
        scene: &mut SceneTarget<'a>,
        bounds: Rect,
    ) -> KludgineResult<()>;

    async fn update_style<'a>(
        &mut self,
        scene: &mut SceneTarget<'a>,
        inherited_style: &Style,
    ) -> KludgineResult<()> {
        self.compute_effective_style(inherited_style, scene);
        Ok(())
    }

    async fn content_size<'a>(
        &self,
        maximum_size: &Size,
        scene: &mut SceneTarget<'a>,
    ) -> KludgineResult<Size>;

    async fn hovered_at(&mut self, window_position: Point) -> KludgineResult<()> {
        self.base_view_mut().hovered_at(window_position)
    }

    async fn unhovered(&mut self) -> KludgineResult<()> {
        self.base_view_mut().unhovered()
    }
}

#[derive(Default, Clone, Debug)]
pub struct BaseView {
    pub style: Style,
    pub hover_style: Style,
    pub effective_style: EffectiveStyle,
    pub layout: Layout,
    pub bounds: Rect,
    pub mouse_status: Option<MouseStatus>,
}

impl BaseView {
    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    pub fn layout_within(&mut self, content_size: &Size, bounds: Rect) -> KludgineResult<()> {
        self.bounds = self.layout.compute_bounds(content_size, bounds);
        Ok(())
    }

    pub fn hovered_at(&mut self, window_position: Point) -> KludgineResult<()> {
        self.mouse_status = Some(MouseStatus::Hovered(window_position));
        Ok(())
    }

    pub fn unhovered(&mut self) -> KludgineResult<()> {
        self.mouse_status = {
            match &self.mouse_status {
                // This is written this way because when we implement mouse down state, this will need to be expanded to track mouse up properly
                Some(status) => match status {
                    MouseStatus::Hovered(_) => None,
                },
                None => None,
            }
        };
        Ok(())
    }
}

#[async_trait]
pub trait ViewCore: std::fmt::Debug + Sync + Send {
    fn base_view(&self) -> &BaseView;
    fn base_view_mut(&mut self) -> &mut BaseView;

    fn bounds(&self) -> Rect {
        self.base_view().bounds
    }

    fn compute_effective_style(&mut self, inherited_style: &Style, scene: &mut SceneTarget) {
        self.base_view_mut().effective_style = self
            .current_style()
            .inherit_from(inherited_style)
            .effective_style(scene);
    }

    fn current_style(&self) -> Style {
        let base_view = self.base_view();
        match &base_view.mouse_status {
            Some(mouse_status) => match mouse_status {
                MouseStatus::Hovered(_) => base_view.hover_style.inherit_from(&base_view.style),
            },
            None => self.base_view().style.clone(),
        }
    }
}

pub trait ViewBuilder {
    fn build(&self) -> KludgineResult<KludgineHandle<Box<dyn View>>>;
}

impl<T> ViewBuilder for T
where
    T: View + Clone + Sized + 'static,
{
    fn build(&self) -> KludgineResult<KludgineHandle<Box<dyn View>>> {
        Ok(Arc::new(RwLock::new(Box::new(self.clone()))))
    }
}

pub trait ViewCoreBuilder {
    fn with_style(&mut self, style: Style) -> &mut Self;
    fn with_hover_style(&mut self, style: Style) -> &mut Self;
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
    fn with_hover_style(&mut self, style: Style) -> &mut Self {
        self.base_view_mut().hover_style = style;
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
