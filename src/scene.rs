use super::{
    math::{Point, Size},
    sprite::RenderedSprite,
    text::{Font, Text},
    timing::Moment,
};
use std::{collections::HashSet, time::Duration};
use winit::event::VirtualKeyCode;

pub(crate) enum Element {
    Sprite(RenderedSprite),
    Text(Text),
}

pub struct Scene {
    pub pressed_keys: HashSet<VirtualKeyCode>,
    pub(crate) scale_factor: f32,
    pub(crate) size: Size,
    pub(crate) elements: Vec<Element>,
    now: Option<Moment>,
    elapsed: Option<Duration>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            scale_factor: 1.0,
            size: Size::default(),
            pressed_keys: HashSet::new(),
            now: None,
            elapsed: None,
            elements: Vec::new(),
        }
    }

    pub(crate) fn start_frame(&mut self) {
        let last_start = self.now;
        self.now = Some(Moment::now());
        self.elapsed = match last_start {
            Some(last_start) => self.now().checked_duration_since(&last_start),
            None => None,
        };
        self.elements.clear();
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn now(&self) -> Moment {
        self.now.expect("now() called without starting a frame")
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.elapsed
    }

    pub fn is_initial_frame(&self) -> bool {
        self.elapsed.is_none()
    }

    pub fn render_text_at<S: Into<String>>(
        &mut self,
        text: S,
        font: &Font,
        size: f32,
        location: Point,
        max_width: Option<f32>,
    ) {
        self.elements.push(Element::Text(Text::new(
            font.clone(),
            size,
            text.into(),
            location,
            max_width,
        )));
    }
}
