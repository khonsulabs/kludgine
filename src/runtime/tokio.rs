use crossbeam::sync::ShardedLock;
use futures::future::Future;
use lazy_static::lazy_static;

lazy_static! {
    pub(crate) static ref GLOBAL_THREAD_POOL: ShardedLock<Option<tokio::runtime::Runtime>> =
        ShardedLock::new(None);
}

pub fn initialize() {
    let mut pool_guard = GLOBAL_THREAD_POOL
        .write()
        .expect("Error locking global thread pool");
    assert!(pool_guard.is_none());
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
}
