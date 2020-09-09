use crate::{
    event::MouseButton,
    math::{Point, Scaled},
};

#[derive(Clone, Debug)]
pub enum ControlEvent {
    Clicked {
        button: MouseButton,
        window_position: Point<f32, Scaled>,
    },
}
