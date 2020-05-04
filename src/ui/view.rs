use crate::{
    math::{Point, Rect, Size},
    style::{EffectiveStyle, Layout, Style},
    KludgineResult,
};

#[derive(Clone, Debug)]
pub enum MouseStatus {
    Hovered(Point),
    Activated(Point),
}

#[derive(Default, Clone, Debug)]
pub struct BaseView {
    pub style: Style,
    pub hover_style: Style,
    pub activated_style: Style,
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
                    MouseStatus::Activated(_) => None,
                },
                None => None,
            }
        };
        Ok(())
    }

    pub fn activated_at(&mut self, window_position: Point) -> KludgineResult<()> {
        self.mouse_status = Some(MouseStatus::Hovered(window_position));
        Ok(())
    }

    pub fn deactivated(&mut self) -> KludgineResult<()> {
        self.mouse_status = {
            match &self.mouse_status {
                // This is written this way because when we implement mouse down state, this will need to be expanded to track mouse up properly
                Some(status) => match status {
                    MouseStatus::Hovered(_) => None,
                    MouseStatus::Activated(location) => Some(MouseStatus::Hovered(*location)),
                },
                None => None,
            }
        };
        Ok(())
    }
}
