use futures::future::Future;
use wasm_bindgen::JsValue;

impl super::Runtime {
    /// Spawns an async task.
    pub fn spawn<Fut: Future<Output = T> + Send + 'static, T: Send + 'static>(future: Fut) {
        drop(wasm_bindgen_futures::future_to_promise(async move {
            drop(future.await);
            Ok(JsValue::NULL)
        }));
    }

    /// Executes a future in a blocking-safe manner.
    pub fn block_on<Fut: Future<Output = R> + Send + 'static, R: Send + Sync + 'static>(
        future: Fut,
    ) {
        wasm_bindgen_futures::spawn_local(async move {
            future.await;
        });
    }
}
