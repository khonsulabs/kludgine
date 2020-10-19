use crate::style::StyleComponent;
use std::fmt::Debug;

pub trait AnyStyleComponent<Unit>: StyleComponent<Unit> + Send + Sync + Debug + 'static {
    fn as_any(&self) -> &'_ dyn std::any::Any;
    fn clone_to_style_component(&self) -> Box<dyn AnyStyleComponent<Unit>>;
}

impl<T: StyleComponent<Unit> + Clone, Unit: Send + Sync + Debug + 'static> AnyStyleComponent<Unit>
    for T
{
    fn as_any(&self) -> &'_ dyn std::any::Any {
        self
    }

    fn clone_to_style_component(&self) -> Box<dyn AnyStyleComponent<Unit>> {
        Box::new(self.clone())
    }
}
