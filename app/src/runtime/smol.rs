use std::time::Duration;

use futures::future::Future;
use kludgine_core::flume;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use smol_timeout::TimeoutExt;

lazy_static! {
    pub static ref GLOBAL_THREAD_POOL: RwLock<Option<smol::Executor<'static>>> = RwLock::new(None);
}

pub fn initialize() {
    {
        let mut pool_guard = GLOBAL_THREAD_POOL.write();
        if pool_guard.is_some() {
            return;
        }

        let executor = smol::Executor::new();
        *pool_guard = Some(executor);
    }

    // Launch a thread pool
    std::thread::spawn(|| {
        let (signal, shutdown) = flume::unbounded::<()>();

        easy_parallel::Parallel::new()
            // Run four executor threads.
            .each(0..4, |_| {
                #[allow(clippy::await_holding_lock)] // this is an rwlock, not a mutex.
                futures::executor::block_on(async {
                    let guard = GLOBAL_THREAD_POOL.read();
                    let executor = guard.as_ref().unwrap();
                    executor.run(shutdown.recv_async()).await
                })
            })
            // Run the main future on the current thread.
            .finish(|| {});

        drop(signal);
    });
}

impl super::Runtime {
    /// Spawns an async task.
    pub fn spawn<Fut: Future<Output = T> + Send + 'static, T: Send + 'static>(future: Fut) {
        let guard = GLOBAL_THREAD_POOL.read();
        let executor = guard.as_ref().unwrap();
        executor.spawn(future).detach();
    }

    /// Executes a future in a blocking-safe manner.
    pub fn block_on<'a, Fut: Future<Output = R> + Send + 'a, R: Send + Sync + 'a>(
        future: Fut,
    ) -> R {
        futures::executor::block_on(future)
    }

    /// Executes `future` for up to `duration`. If a timeout occurs, `None` is
    /// returned.
    pub async fn timeout<F: Future<Output = T> + Send, T: Send>(
        future: F,
        duration: Duration,
    ) -> Option<T> {
        future.timeout(duration).await
    }
}
