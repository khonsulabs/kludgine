use crate::internal_prelude::*;
use glutin::{event_loop::EventLoop, window::WindowBuilder, PossiblyCurrent, WindowedContext};
use std::sync::Once;
static LOAD_SUPPORT: Once = Once::new();

pub struct Window {
    context: WindowedContext<PossiblyCurrent>,
}

impl Window {
    pub(crate) fn new(wb: WindowBuilder, event_loop: &EventLoop<()>) -> Self {
        let windowed_context = glutin::ContextBuilder::new()
            .build_windowed(wb, &event_loop)
            .unwrap();
        let context = unsafe { windowed_context.make_current().unwrap() };

        LOAD_SUPPORT.call_once(|| gl::load_with(|s| context.get_proc_address(s) as *const _));

        Self { context }
    }

    pub fn size(&self) -> Size2d {
        let inner_size = self.context.window().inner_size();
        Size2d::new(inner_size.width as f32, inner_size.height as f32)
    }
}
