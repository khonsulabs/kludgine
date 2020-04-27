use crate::{
    math::{Dimension, Point, Rect, Size, Surround},
    scene::SceneTarget,
    style::{Color, EffectiveStyle, Layout, Style, Weight},
    KludgineResult,
};

pub trait View: std::fmt::Debug {
    fn render(&self, scene: &mut SceneTarget) -> KludgineResult<()>;
    fn layout_within(&mut self, scene: &mut SceneTarget, bounds: Rect) -> KludgineResult<()>;
    fn update_style(
        &mut self,
        scene: &mut SceneTarget,
        inherited_style: &Style,
    ) -> KludgineResult<()>;
    fn content_size(&self, maximum_size: &Size, scene: &mut SceneTarget) -> KludgineResult<Size>;
}

#[derive(Default, Clone, Debug)]
pub struct BaseView {
    pub style: Style,
    pub effective_style: EffectiveStyle,
    pub layout: Layout,
    pub bounds: Rect,
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
