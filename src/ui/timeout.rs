use crate::{
    runtime::Runtime,
    ui::{Entity, InteractiveComponent},
    Handle,
};
use std::time::Duration;

#[derive(Clone)]
pub struct Timeout<T>
where
    T: InteractiveComponent,
{
    duration: Duration,
    target: Entity<T>,
    pending_send: Handle<Option<T::Command>>,
}

impl<T> Timeout<T>
where
    T: InteractiveComponent + 'static,
{
    pub fn new(duration: Duration, target: Entity<T>) -> Self {
        Self {
            duration,
            target,
            pending_send: Default::default(),
        }
    }

    pub async fn send(&self, command: T::Command) {
        let mut pending_send = self.pending_send.write().await;
        if pending_send.is_none() {
            *pending_send = Some(command);
            let duration = self.duration;
            let target = self.target.clone();
            let pending_send = self.pending_send.clone();
            Runtime::spawn(async move {
                futures_timer::Delay::new(duration).await;
                let command = {
                    let mut pending_send = pending_send.write().await;
                    pending_send.take()
                }
                .unwrap();
                let _ = target.send(command).await;
            })
            .detach();
        } else {
            // There is already a task waiting to send the value, just replace it with the newest value.
            *pending_send = Some(command);
        }
    }
}
