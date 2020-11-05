use std::fmt::Debug;

use euclid::Length;

use crate::{
    math::{Point, Raw, Scale, Scaled, Surround},
    style::{ColorPair, Style, StyleComponent, UnscaledStyleComponent},
    window::event::MouseButton,
};

#[derive(Clone, Debug)]
pub enum ControlEvent {
    Clicked {
        button: MouseButton,
        window_position: Point<f32, Scaled>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ControlPadding<Unit>(pub Surround<f32, Unit>);

impl StyleComponent<Scaled> for ControlPadding<Scaled> {
    fn scale(&self, scale: Scale<f32, Scaled, Raw>, destination: &mut Style<Raw>) {
        destination.push(ControlPadding(self.0 * scale))
    }
}

impl StyleComponent<Raw> for ControlPadding<Raw> {
    fn scale(&self, _scale: Scale<f32, Raw, Raw>, map: &mut Style<Raw>) {
        map.push(ControlPadding(self.0));
    }
}

#[derive(Debug, Clone)]
pub struct Border {
    pub width: Length<f32, Scaled>,
    pub color: ColorPair,
}

impl Border {
    pub fn new(width: f32, color: ColorPair) -> Self {
        Self {
            width: Length::new(width),
            color,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ComponentBorder {
    pub left: Option<Border>,
    pub top: Option<Border>,
    pub right: Option<Border>,
    pub bottom: Option<Border>,
}

impl ComponentBorder {
    pub fn uniform(border: Border) -> Self {
        Self {
            left: Some(border.clone()),
            top: Some(border.clone()),
            right: Some(border.clone()),
            bottom: Some(border),
        }
    }

    pub fn with_left(mut self, left: Border) -> Self {
        self.left = Some(left);
        self
    }

    pub fn with_right(mut self, right: Border) -> Self {
        self.right = Some(right);
        self
    }

    pub fn with_bottom(mut self, bottom: Border) -> Self {
        self.bottom = Some(bottom);
        self
    }

    pub fn with_top(mut self, top: Border) -> Self {
        self.top = Some(top);
        self
    }
}

impl UnscaledStyleComponent<Scaled> for ComponentBorder {}
