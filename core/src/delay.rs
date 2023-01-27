#[cfg(target_arch = "wasm32")]
mod implementation {
    use std::time::Duration;

    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    pub struct Delay;

    impl Delay {
        pub async fn new(duration: Duration) {
            let (tx, rx) = flume::bounded(1);

            // TODO was having trouble getting gloo_timers working, it seems very heavy.
            // This seems to be an OK alternative
            {
                let closure = Closure::once_into_js(move || {
                    tx.try_send(()).unwrap();
                });
                web_sys::window()
                    .unwrap()
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        closure.as_ref().unchecked_ref(),
                        duration.as_millis() as i32,
                    )
                    .unwrap();
            }

            rx.recv().await.unwrap()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod implementation {
    pub use futures_timer::Delay;
}

pub use implementation::*;
