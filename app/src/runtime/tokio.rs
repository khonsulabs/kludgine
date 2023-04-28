#![cfg(not(feature = "smol"))]

use std::time::Duration;

use futures::future::Future;
use lazy_static::lazy_static;
use parking_lot::RwLock;

lazy_static! {
    pub(crate) static ref GLOBAL_THREAD_POOL: RwLock<Option<tokio::runtime::Runtime>> =
        RwLock::new(None);
}

pub fn initialize() {
    let mut pool_guard = GLOBAL_THREAD_POOL.write();
    if pool_guard.is_some() {
        return;
    }

    let executor = tokio::runtime::Runtime::new().unwrap();
    *pool_guard = Some(executor);
}

impl super::Runtime {
    /// Spawns an async task.
    pub fn spawn<Fut: Future<Output = T> + Send + 'static, T: Send + 'static>(
        future: Fut,
    ) -> tokio::task::JoinHandle<T> {
        let guard = GLOBAL_THREAD_POOL.read();
        let executor = guard.as_ref().unwrap();
        executor.spawn(future)
    }

    /// Executes a future in a blocking-safe manner.
    pub fn block_on<'a, Fut: Future<Output = R> + Send + 'a, R: Send + Sync + 'a>(
        future: Fut,
    ) -> R {
        let guard = GLOBAL_THREAD_POOL.read();
        let executor = guard.as_ref().unwrap();
        executor.block_on(future)
    }

    /// Executes `future` for up to `duration`. If a timeout occurs, `None` is
    /// returned.
    pub async fn timeout<F: Future<Output = T>, T: Send>(
        future: F,
        duration: Duration,
    ) -> Option<T> {
        tokio::time::timeout(duration, future).await.ok()
    }
}
