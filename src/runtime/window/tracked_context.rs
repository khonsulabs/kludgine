use glutin::{NotCurrent, PossiblyCurrent, WindowedContext};
use std::ops::Deref;

pub enum TrackedContext {
    Current(WindowedContext<PossiblyCurrent>),
    NotCurrent(WindowedContext<NotCurrent>),
}

impl Deref for TrackedContext {
    type Target = WindowedContext<PossiblyCurrent>;

    fn deref(&self) -> &Self::Target {
        match self {
            TrackedContext::Current(ctx) => ctx,
            TrackedContext::NotCurrent(_) => panic!(),
        }
    }
}

impl TrackedContext {
    pub fn window(&self) -> &glutin::window::Window {
        match self {
            TrackedContext::Current(ctx) => ctx.window(),
            TrackedContext::NotCurrent(ctx) => ctx.window(),
        }
    }

    pub fn make_current(self) -> Self {
        match self {
            TrackedContext::Current(_) => {
                panic!("Attempting to make the current context current again")
            }
            TrackedContext::NotCurrent(ctx) => {
                TrackedContext::Current(unsafe { ctx.make_current() }.unwrap())
            }
        }
    }

    pub fn treat_as_not_current(self) -> Self {
        match self {
            TrackedContext::Current(ctx) => {
                TrackedContext::NotCurrent(unsafe { ctx.treat_as_not_current() })
            }
            TrackedContext::NotCurrent(ctx) => TrackedContext::NotCurrent(ctx),
        }
    }
}
