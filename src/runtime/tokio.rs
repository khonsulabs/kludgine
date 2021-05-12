#![cfg(not(feature = "smol-rt"))]

use futures::future::Future;
use lazy_static::lazy_static;
use std::sync::RwLock;
use std::time::Duration;

lazy_static! {
    pub(crate) static ref GLOBAL_THREAD_POOL: RwLock<Option<tokio::runtime::Runtime>> =
        RwLock::new(None);
}

pub fn initialize() {
    let mut pool_guard = GLOBAL_THREAD_POOL
        .write()
        .expect("Error locking global thread pool");
    if pool_guard.is_some() {
        return;
    }

    let executor = tokio::runtime::Runtime::new().unwrap();
    *pool_guard = Some(executor);
}

impl super::Runtime {
    pub fn spawn<Fut: Future<Output = T> + Send + 'static, T: Send + 'static>(
        future: Fut,
    ) -> tokio::task::JoinHandle<T> {
        let guard = GLOBAL_THREAD_POOL.read().expect("Error getting runtime");
        let executor = guard.as_ref().unwrap();
        executor.spawn(future)
    }

    pub fn block_on<'a, Fut: Future<Output = R> + Send + 'a, R: Send + Sync + 'a>(
        future: Fut,
    ) -> R {
        let guard = GLOBAL_THREAD_POOL.read().expect("Error getting runtime");
        let executor = guard.as_ref().unwrap();
        executor.block_on(future)
    }

    pub async fn timeout<F: Future<Output = T>, T: Send>(
        future: F,
        duration: Duration,
    ) -> Option<T> {
        tokio::time::timeout(duration, future).await.ok()
    }
}
