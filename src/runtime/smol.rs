use crate::{
    application::Application,
    style::theme::SystemTheme,
    window::{RuntimeWindow, Window, WindowBuilder},
    KludgineResult,
};
use crossbeam::{
    channel::{unbounded, Receiver, Sender, TryRecvError},
    sync::ShardedLock,
};
use futures::future::Future;
use lazy_static::lazy_static;
use platforms::target::{OS, TARGET_OS};
use std::time::Duration;

lazy_static! {
    pub(crate) static ref GLOBAL_THREAD_POOL: ShardedLock<Option<smol::Executor<'static>>> =
        ShardedLock::new(None);
}

pub fn initialize() {
    {
        let mut pool_guard = GLOBAL_THREAD_POOL
            .write()
            .expect("Error locking global thread pool");
        assert!(pool_guard.is_none());
        let executor = smol::Executor::new();
        *pool_guard = Some(executor);
    }

    // Launch a thread pool
    std::thread::spawn(|| {
        let (signal, shutdown) = async_channel::unbounded::<()>();

        easy_parallel::Parallel::new()
            // Run four executor threads.
            .each(0..4, |_| {
                futures::executor::block_on(async {
                    let guard = GLOBAL_THREAD_POOL.read().unwrap();
                    let executor = guard.as_ref().unwrap();
                    executor.run(shutdown.recv()).await
                })
            })
            // Run the main future on the current thread.
            .finish(|| {});

        signal.close();
    });
}

impl super::Runtime {
    pub fn spawn<Fut: Future<Output = T> + Send + 'static, T: Send + 'static>(future: Fut) {
        let guard = GLOBAL_THREAD_POOL.read().expect("Error getting runtime");
        let executor = guard.as_ref().unwrap();
        executor.spawn(future).detach()
    }

    pub fn block_on<'a, Fut: Future<Output = R> + Send + 'a, R: Send + Sync + 'a>(
        future: Fut,
    ) -> R {
        futures::executor::block_on(future)
    }
}
