use super::request::{RuntimeEvent, RuntimeRequest};
use crate::internal_prelude::*;
use futures::executor::ThreadPool;
use lazy_static::lazy_static;
use std::sync::Mutex;

pub trait EventProcessor: Send + Sync {
    fn process_event(
        &mut self,
        event_loop: &glutin::event_loop::EventLoopWindowTarget<()>,
        event: glutin::event::Event<()>,
        control_flow: &mut glutin::event_loop::ControlFlow,
    );
}
lazy_static! {
    pub(crate) static ref GLOBAL_RUNTIME_SENDER: Mutex<Option<Sender<RuntimeRequest>>> =
        { Mutex::new(None) };
    pub(crate) static ref GLOBAL_RUNTIME_RECEIVER: Mutex<Option<Receiver<RuntimeEvent>>> =
        { Mutex::new(None) };
    pub(crate) static ref GLOBAL_EVENT_HANDLER: Mutex<Option<Box<dyn EventProcessor>>> =
        Mutex::new(None);
    pub(crate) static ref GLOBAL_THREAD_POOL: Mutex<Option<ThreadPool>> = Mutex::new(None);
}
