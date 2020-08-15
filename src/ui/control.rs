use crate::{
    event::MouseButton,
    math::{Point, Points},
};

#[derive(Clone, Debug)]
pub enum ControlEvent {
    Clicked {
        button: MouseButton,
        window_position: Point<Points>,
    },
}
