use crate::event::MouseButton;

#[derive(Clone, Debug)]
pub enum ControlEvent {
    Clicked(MouseButton),
}
