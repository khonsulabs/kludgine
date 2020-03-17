use super::{
    request::{RuntimeEvent, RuntimeRequest},
    CloseResponse, Runtime,
};
use crate::application::Application;
use crate::internal_prelude::*;
use futures::executor::ThreadPool;
use lazy_static::lazy_static;
use std::sync::Mutex;

pub trait EventProcessor: Send + Sync {
    fn process_event(
        &mut self,
        event: glutin::event::Event<()>,
        control_flow: &mut glutin::event_loop::ControlFlow,
        display: &glium::Display,
    );
}
lazy_static! {
    pub(crate) static ref GLOBAL_RUNTIME_SENDER: Mutex<Option<mpsc::UnboundedSender<RuntimeRequest>>> =
        { Mutex::new(None) };
    pub(crate) static ref GLOBAL_RUNTIME_RECEIVER: Mutex<Option<mpsc::UnboundedReceiver<RuntimeEvent>>> =
        { Mutex::new(None) };
    pub(crate) static ref GLOBAL_EVENT_HANDLER: Mutex<Option<Box<dyn EventProcessor>>> =
        Mutex::new(None);
    pub(crate) static ref GLOBAL_THREAD_POOL: Mutex<Option<ThreadPool>> = Mutex::new(None);
}
