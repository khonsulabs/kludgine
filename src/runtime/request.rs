use super::threading::GLOBAL_RUNTIME_SENDER;
use crate::internal_prelude::*;

pub(crate) enum RuntimeRequest {
    // UpdateScene,
// NewWindow {
//     notify: oneshot::Sender<KludgineResult<NewWindowResponse>>,
// },
}

impl RuntimeRequest {
    pub async fn send(self) -> KludgineResult<()> {
        let sender = GLOBAL_RUNTIME_SENDER.lock().expect("Error locking mutex");
        sender
            .as_ref()
            .expect("Runtime not initialized")
            .send(self)
            .await
            .expect("Error sending runtime request");
        Ok(())
    }
}
