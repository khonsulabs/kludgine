use crate::ui::BaseComponent;

pub struct Node {
    pub(crate) component: Box<dyn BaseComponent>,
}
