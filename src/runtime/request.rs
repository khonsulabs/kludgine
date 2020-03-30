use super::threading::GLOBAL_RUNTIME_SENDER;
use crate::internal_prelude::*;
use crate::window::Window;

pub(crate) enum RuntimeRequest {
    // UpdateScene,
    // NewWindow {
    //     notify: oneshot::Sender<KludgineResult<NewWindowResponse>>,
    // },
    OpenWindow {
        builder: glutin::window::WindowBuilder,
        window: Box<dyn Window>,
    },
    Quit,
}

impl RuntimeRequest {
    pub async fn send(self) -> KludgineResult<()> {
        let sender: Sender<RuntimeRequest> = {
            let guard = GLOBAL_RUNTIME_SENDER.lock().expect("Error locking mutex");
            match *guard {
                Some(ref sender) => sender.clone(),
                None => panic!("Uninitialized runtime"),
            }
        };
        sender.send(self).unwrap_or_default();
        Ok(())
    }
}

pub(crate) enum RuntimeEvent {
    Running,
}
